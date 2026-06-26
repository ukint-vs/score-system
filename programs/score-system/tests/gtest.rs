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

#[tokio::test]
async fn attester_records_score() {
    let env = GtestEnv::system_default();
    env.system()
        .mint_to(DEFAULT_USER_BOB, DEFAULT_USERS_INITIAL_BALANCE);
    let code_id = env.system().submit_code(::score_system::WASM_BINARY);
    let owner = ActorId::from(DEFAULT_USER_ALICE);
    let attester = ActorId::from(DEFAULT_USER_BOB);
    let subject = actor(10);

    let program = env
        .deploy::<::score_system_client::ScoreSystemClientProgram>(code_id, b"salt".to_vec())
        .new(vec![attester])
        .await
        .unwrap();

    let mut service = program.score();
    let mut events = service.listen().await.unwrap();

    let config = service.get_config().await.unwrap();
    assert_eq!(owner, config.owner);
    assert_eq!(vec![attester, owner], config.score_attesters);

    service
        .record_score(request(subject))
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
