#![no_std]

use core::cell::RefCell;
use sails_rs::prelude::{collections::*, *};

pub type Timestamp = u64;
pub type ScoreId = u64;

const MAX_REASON_CODE_BYTES: usize = 64;
const MAX_HISTORY_LIMIT: u32 = 100;

#[sails_rs::sails_type]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScoreDimensions {
    pub proposal_quality: u8,
    pub delivery_reliability: u8,
    pub integration_count: u8,
    pub past_impact_score: u8,
    pub community_signal: u8,
}

#[sails_rs::sails_type]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordScoreReq {
    pub actor: ActorId,
    pub dimensions: ScoreDimensions,
    pub confidence: u8,
    pub evidence_hash: [u8; 32],
    pub reason_code: String,
}

#[sails_rs::sails_type]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreSnapshot {
    pub id: ScoreId,
    pub actor: ActorId,
    pub dimensions: ScoreDimensions,
    pub overall: u8,
    pub confidence: u8,
    pub evidence_hash: [u8; 32],
    pub reason_code: String,
    pub formula_version: u32,
    pub attester: ActorId,
    pub recorded_at: Timestamp,
}

#[sails_rs::sails_type]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScoreHistoryPage {
    pub items: Vec<ScoreSnapshot>,
    pub next_cursor: Option<ScoreId>,
}

#[sails_rs::sails_type]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreConfig {
    pub owner: ActorId,
    pub score_attesters: Vec<ActorId>,
    pub formula_version: u32,
    pub next_score_id: ScoreId,
    pub paused: bool,
}

#[sails_rs::sails_type]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScoreError {
    Unauthorized,
    Paused,
    InvalidActor,
    InvalidDimension,
    InvalidConfidence,
    InvalidEvidenceHash,
    EmptyReasonCode,
    ReasonCodeTooLong,
    LimitTooLarge,
    ScoreIdOverflow,
    InvalidFormulaVersion,
}

#[sails_rs::sails_type]
#[sails_rs::event]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScoreEvents {
    FormulaVersionUpdated {
        old_version: u32,
        new_version: u32,
    },
    Paused,
    ScoreAttesterAdded {
        attester: ActorId,
    },
    ScoreAttesterRemoved {
        attester: ActorId,
    },
    ScoreRecorded {
        id: ScoreId,
        actor: ActorId,
        overall: u8,
        confidence: u8,
        evidence_hash: [u8; 32],
        reason_code: String,
        formula_version: u32,
        attester: ActorId,
        recorded_at: Timestamp,
    },
    Unpaused,
}

#[derive(Clone)]
pub struct ScoreState {
    pub owner: ActorId,
    pub score_attesters: BTreeSet<ActorId>,
    pub formula_version: u32,
    pub next_score_id: ScoreId,
    pub latest_by_actor: BTreeMap<ActorId, ScoreId>,
    pub snapshots: BTreeMap<ScoreId, ScoreSnapshot>,
    pub history_by_actor: BTreeMap<ActorId, Vec<ScoreId>>,
    pub paused: bool,
}

impl ScoreState {
    pub fn new(owner: ActorId, initial_attesters: Vec<ActorId>) -> Self {
        let mut score_attesters = BTreeSet::new();
        score_attesters.insert(owner);
        for attester in initial_attesters {
            if attester != ActorId::default() {
                score_attesters.insert(attester);
            }
        }

        Self {
            owner,
            score_attesters,
            formula_version: 1,
            next_score_id: 1,
            latest_by_actor: BTreeMap::new(),
            snapshots: BTreeMap::new(),
            history_by_actor: BTreeMap::new(),
            paused: false,
        }
    }
}

impl Default for ScoreState {
    fn default() -> Self {
        Self::new(ActorId::default(), Vec::new())
    }
}

pub struct Score<S: StateMut<Item = ScoreState, Error = Infallible> = RefCell<ScoreState>> {
    state: S,
}

impl<S: StateMut<Item = ScoreState, Error = Infallible>> Score<S> {
    pub fn new(state: S) -> Self {
        Self { state }
    }

    fn require_owner(&self, caller: ActorId) -> Result<(), ScoreError> {
        if self.state.get().owner == caller {
            Ok(())
        } else {
            Err(ScoreError::Unauthorized)
        }
    }

    fn validate_record(state: &ScoreState, req: &RecordScoreReq) -> Result<(), ScoreError> {
        if state.paused {
            return Err(ScoreError::Paused);
        }
        if req.actor == ActorId::default() {
            return Err(ScoreError::InvalidActor);
        }
        if !dimensions_valid(&req.dimensions) {
            return Err(ScoreError::InvalidDimension);
        }
        if req.confidence > 100 {
            return Err(ScoreError::InvalidConfidence);
        }
        if req.evidence_hash == [0; 32] {
            return Err(ScoreError::InvalidEvidenceHash);
        }
        if req.reason_code.is_empty() {
            return Err(ScoreError::EmptyReasonCode);
        }
        if req.reason_code.len() > MAX_REASON_CODE_BYTES {
            return Err(ScoreError::ReasonCodeTooLong);
        }
        Ok(())
    }
}

#[sails_rs::service(events = ScoreEvents)]
impl<S: StateMut<Item = ScoreState, Error = Infallible>> Score<S> {
    #[export]
    pub fn record_score(&mut self, req: RecordScoreReq) -> Result<ScoreId, ScoreError> {
        let attester = Syscall::message_source();
        let snapshot;
        {
            let mut state = self.state.get_mut();
            if !state.score_attesters.contains(&attester) {
                return Err(ScoreError::Unauthorized);
            }
            Score::<S>::validate_record(&state, &req)?;

            let id = state.next_score_id;
            state.next_score_id = id.checked_add(1).ok_or(ScoreError::ScoreIdOverflow)?;

            let overall = overall(&req.dimensions);
            snapshot = ScoreSnapshot {
                id,
                actor: req.actor,
                dimensions: req.dimensions,
                overall,
                confidence: req.confidence,
                evidence_hash: req.evidence_hash,
                reason_code: req.reason_code,
                formula_version: state.formula_version,
                attester,
                recorded_at: Syscall::block_timestamp(),
            };

            state.latest_by_actor.insert(snapshot.actor, id);
            state
                .history_by_actor
                .entry(snapshot.actor)
                .or_default()
                .push(id);
            state.snapshots.insert(id, snapshot.clone());
        }

        self.emit_event(ScoreEvents::ScoreRecorded {
            id: snapshot.id,
            actor: snapshot.actor,
            overall: snapshot.overall,
            confidence: snapshot.confidence,
            evidence_hash: snapshot.evidence_hash,
            reason_code: snapshot.reason_code,
            formula_version: snapshot.formula_version,
            attester: snapshot.attester,
            recorded_at: snapshot.recorded_at,
        })
        .unwrap();

        Ok(snapshot.id)
    }

    #[export]
    pub fn get_score(&self, actor: ActorId) -> Option<ScoreSnapshot> {
        self.get_score_snapshot(actor)
    }

    #[export]
    pub fn get_score_snapshot(&self, actor: ActorId) -> Option<ScoreSnapshot> {
        let state = self.state.get();
        let id = state.latest_by_actor.get(&actor)?;
        state.snapshots.get(id).cloned()
    }

    #[export]
    pub fn get_score_history(
        &self,
        actor: ActorId,
        cursor: Option<ScoreId>,
        limit: u32,
    ) -> Result<ScoreHistoryPage, ScoreError> {
        if limit == 0 || limit > MAX_HISTORY_LIMIT {
            return Err(ScoreError::LimitTooLarge);
        }

        let state = self.state.get();
        let ids = match state.history_by_actor.get(&actor) {
            Some(ids) => ids,
            None => return Ok(ScoreHistoryPage::default()),
        };

        // ponytail: O(n) history scan is fine for Stage 1's ~30 actors; add a cursor index if this grows.
        let eligible: Vec<ScoreId> = ids
            .iter()
            .rev()
            .copied()
            .filter(|id| cursor.map(|cursor| *id < cursor).unwrap_or(true))
            .collect();
        let items: Vec<ScoreSnapshot> = eligible
            .iter()
            .take(limit as usize)
            .filter_map(|id| state.snapshots.get(id).cloned())
            .collect();
        let next_cursor = if eligible.len() > limit as usize {
            items.last().map(|snapshot| snapshot.id)
        } else {
            None
        };

        Ok(ScoreHistoryPage { items, next_cursor })
    }

    #[export]
    pub fn add_score_attester(&mut self, attester: ActorId) -> Result<(), ScoreError> {
        self.require_owner(Syscall::message_source())?;
        if attester == ActorId::default() {
            return Err(ScoreError::InvalidActor);
        }

        let inserted = self.state.get_mut().score_attesters.insert(attester);
        if inserted {
            self.emit_event(ScoreEvents::ScoreAttesterAdded { attester })
                .unwrap();
        }
        Ok(())
    }

    #[export]
    pub fn remove_score_attester(&mut self, attester: ActorId) -> Result<(), ScoreError> {
        self.require_owner(Syscall::message_source())?;
        let removed = self.state.get_mut().score_attesters.remove(&attester);
        if removed {
            self.emit_event(ScoreEvents::ScoreAttesterRemoved { attester })
                .unwrap();
        }
        Ok(())
    }

    #[export]
    pub fn list_score_attesters(&self) -> Vec<ActorId> {
        self.state.get().score_attesters.iter().copied().collect()
    }

    #[export]
    pub fn pause(&mut self) -> Result<(), ScoreError> {
        self.require_owner(Syscall::message_source())?;
        let changed = {
            let mut state = self.state.get_mut();
            if state.paused {
                false
            } else {
                state.paused = true;
                true
            }
        };
        if changed {
            self.emit_event(ScoreEvents::Paused).unwrap();
        }
        Ok(())
    }

    #[export]
    pub fn unpause(&mut self) -> Result<(), ScoreError> {
        self.require_owner(Syscall::message_source())?;
        let changed = {
            let mut state = self.state.get_mut();
            if state.paused {
                state.paused = false;
                true
            } else {
                false
            }
        };
        if changed {
            self.emit_event(ScoreEvents::Unpaused).unwrap();
        }
        Ok(())
    }

    #[export]
    pub fn update_formula_version(&mut self, new_version: u32) -> Result<(), ScoreError> {
        self.require_owner(Syscall::message_source())?;
        if new_version == 0 {
            return Err(ScoreError::InvalidFormulaVersion);
        }

        let old_version;
        {
            let mut state = self.state.get_mut();
            if state.formula_version == new_version {
                return Ok(());
            }
            old_version = state.formula_version;
            state.formula_version = new_version;
        }

        self.emit_event(ScoreEvents::FormulaVersionUpdated {
            old_version,
            new_version,
        })
        .unwrap();
        Ok(())
    }

    #[export]
    pub fn get_config(&self) -> ScoreConfig {
        let state = self.state.get();
        ScoreConfig {
            owner: state.owner,
            score_attesters: state.score_attesters.iter().copied().collect(),
            formula_version: state.formula_version,
            next_score_id: state.next_score_id,
            paused: state.paused,
        }
    }
}

#[derive(Default)]
pub struct Program {
    score_state: RefCell<ScoreState>,
}

#[sails_rs::program]
impl Program {
    pub fn new(initial_attesters: Vec<ActorId>) -> Self {
        Self {
            score_state: RefCell::new(ScoreState::new(
                Syscall::message_source(),
                initial_attesters,
            )),
        }
    }

    pub fn score(&self) -> Score<&RefCell<ScoreState>> {
        Score::new(&self.score_state)
    }
}

fn dimensions_valid(dimensions: &ScoreDimensions) -> bool {
    dimensions.proposal_quality <= 100
        && dimensions.delivery_reliability <= 100
        && dimensions.integration_count <= 100
        && dimensions.past_impact_score <= 100
        && dimensions.community_signal <= 100
}

fn overall(dimensions: &ScoreDimensions) -> u8 {
    ((dimensions.proposal_quality as u16 * 30
        + dimensions.delivery_reliability as u16 * 25
        + dimensions.integration_count as u16 * 20
        + dimensions.past_impact_score as u16 * 15
        + dimensions.community_signal as u16 * 10)
        / 100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use sails_rs::gstd::services::Service as _;

    fn actor(id: u64) -> ActorId {
        ActorId::from(id)
    }

    fn req(actor: ActorId) -> RecordScoreReq {
        RecordScoreReq {
            actor,
            dimensions: ScoreDimensions {
                proposal_quality: 100,
                delivery_reliability: 80,
                integration_count: 60,
                past_impact_score: 40,
                community_signal: 20,
            },
            confidence: 90,
            evidence_hash: [7; 32],
            reason_code: "manual-review".to_string(),
        }
    }

    fn service(
        owner: ActorId,
        initial_attesters: Vec<ActorId>,
    ) -> Score<&'static RefCell<ScoreState>> {
        let state = Box::leak(Box::new(RefCell::new(ScoreState::new(
            owner,
            initial_attesters,
        ))));
        Score::new(state).expose(0)
    }

    #[test]
    fn constructor_state_includes_owner_and_initial_attesters() {
        let owner = actor(1);
        let attester = actor(2);
        let service = service(owner, vec![attester, ActorId::default(), attester]);

        let config = service.get_config();
        assert_eq!(owner, config.owner);
        assert_eq!(1, config.formula_version);
        assert_eq!(1, config.next_score_id);
        assert!(!config.paused);
        assert_eq!(vec![owner, attester], config.score_attesters);
    }

    #[test]
    fn unauthorized_caller_cannot_record() {
        Syscall::with_message_source(actor(9));
        let mut service = service(actor(1), vec![actor(2)]);

        assert_eq!(
            Err(ScoreError::Unauthorized),
            service.record_score(req(actor(10)))
        );
    }

    #[test]
    fn attester_records_latest_and_history() {
        let attester = actor(2);
        let subject = actor(10);
        Syscall::with_message_source(attester);
        Syscall::with_block_timestamp(77);
        let mut service = service(actor(1), vec![attester]);

        let id = service.record_score(req(subject)).unwrap();

        assert_eq!(1, id);
        let snapshot = service.get_score_snapshot(subject).unwrap();
        assert_eq!(1, snapshot.id);
        assert_eq!(subject, snapshot.actor);
        assert_eq!(70, snapshot.overall);
        assert_eq!(attester, snapshot.attester);
        assert_eq!(77, snapshot.recorded_at);
        assert_eq!(service.get_score(subject), Some(snapshot.clone()));
        assert_eq!(
            ScoreHistoryPage {
                items: vec![snapshot],
                next_cursor: None
            },
            service.get_score_history(subject, None, 10).unwrap()
        );
    }

    #[test]
    fn validation_rejects_bad_record_inputs() {
        let attester = actor(2);
        Syscall::with_message_source(attester);
        let mut service = service(actor(1), vec![attester]);

        let mut bad = req(ActorId::default());
        assert_eq!(Err(ScoreError::InvalidActor), service.record_score(bad));

        bad = req(actor(10));
        bad.confidence = 101;
        assert_eq!(
            Err(ScoreError::InvalidConfidence),
            service.record_score(bad)
        );

        bad = req(actor(10));
        bad.evidence_hash = [0; 32];
        assert_eq!(
            Err(ScoreError::InvalidEvidenceHash),
            service.record_score(bad)
        );

        bad = req(actor(10));
        bad.reason_code = String::new();
        assert_eq!(Err(ScoreError::EmptyReasonCode), service.record_score(bad));

        bad = req(actor(10));
        bad.reason_code = "x".repeat(MAX_REASON_CODE_BYTES + 1);
        assert_eq!(
            Err(ScoreError::ReasonCodeTooLong),
            service.record_score(bad)
        );
    }

    #[test]
    fn history_is_append_only_and_paginated_newest_first() {
        let attester = actor(2);
        let subject = actor(10);
        Syscall::with_message_source(attester);
        let mut service = service(actor(1), vec![attester]);

        for i in 0..3 {
            let mut req = req(subject);
            req.reason_code = format!("review-{i}");
            service.record_score(req).unwrap();
        }

        let page = service.get_score_history(subject, None, 2).unwrap();
        assert_eq!(
            vec![3, 2],
            page.items.iter().map(|item| item.id).collect::<Vec<_>>()
        );
        assert_eq!(Some(2), page.next_cursor);

        let page = service
            .get_score_history(subject, page.next_cursor, 2)
            .unwrap();
        assert_eq!(
            vec![1],
            page.items.iter().map(|item| item.id).collect::<Vec<_>>()
        );
        assert_eq!(None, page.next_cursor);
        assert_eq!(
            Some(3),
            service.get_score(subject).map(|snapshot| snapshot.id)
        );
    }

    #[test]
    fn pause_blocks_recording_but_not_queries() {
        let owner = actor(1);
        let attester = actor(2);
        let subject = actor(10);
        let mut service = service(owner, vec![attester]);

        Syscall::with_message_source(owner);
        service.pause().unwrap();

        Syscall::with_message_source(attester);
        assert_eq!(Err(ScoreError::Paused), service.record_score(req(subject)));
        assert_eq!(None, service.get_score(subject));

        Syscall::with_message_source(owner);
        service.unpause().unwrap();

        Syscall::with_message_source(attester);
        assert_eq!(Ok(1), service.record_score(req(subject)));
    }

    #[test]
    fn owner_admin_methods_work() {
        let owner = actor(1);
        let attester = actor(2);
        let mut service = service(owner, Vec::new());

        Syscall::with_message_source(attester);
        assert_eq!(
            Err(ScoreError::Unauthorized),
            service.add_score_attester(attester)
        );

        Syscall::with_message_source(owner);
        service.add_score_attester(attester).unwrap();
        service.add_score_attester(attester).unwrap();
        assert_eq!(vec![owner, attester], service.list_score_attesters());

        service.update_formula_version(2).unwrap();
        assert_eq!(2, service.get_config().formula_version);
        assert_eq!(
            Err(ScoreError::InvalidFormulaVersion),
            service.update_formula_version(0)
        );

        service.remove_score_attester(attester).unwrap();
        assert_eq!(vec![owner], service.list_score_attesters());
    }
}
