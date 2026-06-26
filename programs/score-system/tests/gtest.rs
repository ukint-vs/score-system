use ::score_system_client::{ScoreSystemClient as _, ScoreSystemClientCtors as _, score::*};
use sails_rs::prelude::futures::StreamExt as _;
use sails_rs::{client::*, prelude::*};

fn actor(id: u64) -> ActorId {
    ActorId::from(id)
}

fn request(subject: ActorId) -> RecordScoreReq {
    RecordScoreReq {
        actor: subject,
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

async fn deploy_with_attester() -> (
    GtestEnv,
    Actor<::score_system_client::ScoreSystemClientProgram, GtestEnv>,
    ActorId,
) {
    let env = GtestEnv::system_default();
    env.system()
        .mint_to(DEFAULT_USER_BOB, DEFAULT_USERS_INITIAL_BALANCE);
    env.system()
        .mint_to(DEFAULT_USER_CHARLIE, DEFAULT_USERS_INITIAL_BALANCE);
    let code_id = env.system().submit_code(::score_system::WASM_BINARY);
    let attester = ActorId::from(DEFAULT_USER_BOB);
    let program = env
        .deploy::<::score_system_client::ScoreSystemClientProgram>(code_id, b"salt".to_vec())
        .new(vec![attester])
        .await
        .unwrap();

    (env, program, attester)
}

#[tokio::test]
async fn attester_records_score() {
    let owner = ActorId::from(DEFAULT_USER_ALICE);
    let subject = actor(10);
    let (_env, program, attester) = deploy_with_attester().await;

    let mut service = program.score();
    let mut events = service.listen().await.unwrap();

    let config = service.get_config().await.unwrap();
    assert_eq!(owner, config.owner);
    assert_eq!(vec![attester, owner], config.score_attesters);

    let req = request(subject);
    let dimensions = req.dimensions.clone();
    service
        .record_score(req)
        .with_actor_id(attester)
        .await
        .unwrap()
        .unwrap();

    let snapshot = service.get_score_snapshot(subject).await.unwrap().unwrap();
    assert_eq!(subject, snapshot.actor);
    assert_eq!(70, snapshot.overall);
    assert_eq!(attester, snapshot.attester);

    let history = service
        .get_score_history(subject, None, 10)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(vec![snapshot.clone()], history.items);
    assert_eq!(None, history.next_cursor);

    let (actor_id, event) = events.next().await.unwrap();
    assert_eq!(program.id(), actor_id);
    assert_eq!(
        events::ScoreEvents::ScoreRecorded {
            id: 1,
            actor: subject,
            dimensions,
            overall: 70,
            confidence: 90,
            evidence_hash: [7; 32],
            reason_code: "manual-review".to_string(),
            formula_version: 1,
            attester,
            recorded_at: snapshot.recorded_at,
        },
        event
    );
}

#[tokio::test]
async fn unauthorized_and_invalid_records_are_rejected() {
    let subject = actor(10);
    let (_env, program, attester) = deploy_with_attester().await;
    let mut service = program.score();

    assert_eq!(
        Err(ScoreError::Unauthorized),
        service
            .record_score(request(subject))
            .with_actor_id(ActorId::from(DEFAULT_USER_CHARLIE))
            .await
            .unwrap()
    );

    let mut bad = request(subject);
    bad.dimensions.proposal_quality = 101;
    assert_eq!(
        Err(ScoreError::InvalidDimension),
        service
            .record_score(bad)
            .with_actor_id(attester)
            .await
            .unwrap()
    );

    let mut bad = request(subject);
    bad.confidence = 101;
    assert_eq!(
        Err(ScoreError::InvalidConfidence),
        service
            .record_score(bad)
            .with_actor_id(attester)
            .await
            .unwrap()
    );

    let mut bad = request(subject);
    bad.evidence_hash = [0; 32];
    assert_eq!(
        Err(ScoreError::InvalidEvidenceHash),
        service
            .record_score(bad)
            .with_actor_id(attester)
            .await
            .unwrap()
    );

    let mut bad = request(subject);
    bad.reason_code.clear();
    assert_eq!(
        Err(ScoreError::EmptyReasonCode),
        service
            .record_score(bad)
            .with_actor_id(attester)
            .await
            .unwrap()
    );

    let mut bad = request(subject);
    bad.reason_code = "x".repeat(65);
    assert_eq!(
        Err(ScoreError::ReasonCodeTooLong),
        service
            .record_score(bad)
            .with_actor_id(attester)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn history_is_append_only_and_paginated() {
    let subject = actor(10);
    let (_env, program, attester) = deploy_with_attester().await;
    let mut service = program.score();

    for i in 0..3 {
        let mut req = request(subject);
        req.reason_code = format!("review-{i}");
        assert_eq!(
            Ok(i + 1),
            service
                .record_score(req)
                .with_actor_id(attester)
                .await
                .unwrap()
        );
    }

    let latest = service.get_score(subject).await.unwrap().unwrap();
    let snapshot = service.get_score_snapshot(subject).await.unwrap().unwrap();
    assert_eq!(latest, snapshot);
    assert_eq!(3, snapshot.id);

    let first_page = service
        .get_score_history(subject, None, 2)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        vec![3, 2],
        first_page
            .items
            .iter()
            .map(|snapshot| snapshot.id)
            .collect::<Vec<_>>()
    );
    assert_eq!(Some(2), first_page.next_cursor);

    let second_page = service
        .get_score_history(subject, first_page.next_cursor, 2)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        vec![1],
        second_page
            .items
            .iter()
            .map(|snapshot| snapshot.id)
            .collect::<Vec<_>>()
    );
    assert_eq!(None, second_page.next_cursor);

    assert_eq!(
        Err(ScoreError::LimitTooLarge),
        service.get_score_history(subject, None, 0).await.unwrap()
    );
    assert_eq!(
        Err(ScoreError::LimitTooLarge),
        service.get_score_history(subject, None, 101).await.unwrap()
    );
}

#[tokio::test]
async fn pause_blocks_writes_but_not_queries() {
    let subject = actor(10);
    let (_env, program, attester) = deploy_with_attester().await;
    let mut service = program.score();

    service
        .record_score(request(subject))
        .with_actor_id(attester)
        .await
        .unwrap()
        .unwrap();
    service.pause().await.unwrap().unwrap();

    assert_eq!(
        Err(ScoreError::Paused),
        service
            .record_score(request(subject))
            .with_actor_id(attester)
            .await
            .unwrap()
    );
    assert!(service.get_score_snapshot(subject).await.unwrap().is_some());

    service.unpause().await.unwrap().unwrap();
    assert_eq!(
        Ok(2),
        service
            .record_score(request(subject))
            .with_actor_id(attester)
            .await
            .unwrap()
    );
}
