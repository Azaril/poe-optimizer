//! Focused dependency-channel tests complement the public graph witness.
use super::*;
use crate::owned_plan::compile::support_fixture as fixture;

#[test]
fn scalar_and_transform_readiness_are_distinct_even_at_the_same_exact_key() {
    let f = fixture::generated_fixture();
    let inputs = fixture::compile_inputs(&f, fixture::target(30, "first"));
    let key = PlanValueKey::Stat {
        entity: ConcreteEntity::Modifier(root(ProviderRoot::Character)),
        stat: fixture::def("separate-channel"),
    };
    for transform_first in [false, true] {
        let mut work = 1000;
        let mut proof = ReadinessProof {
            stages: &inputs.stages,
            index: inputs.definitions.as_ref(),
            values: BTreeMap::new(),
            contributions: BTreeMap::new(),
            transforms: BTreeMap::new(),
            effects: vec![],
            work: &mut work,
        };
        let scalar = BoundEffectTarget::Value { key: key.clone() };
        let transform = BoundEffectTarget::ModifierTransform {
            key: key.clone(),
            operation: ModifierTransformOperation::Multiply,
            order: BoundedInteger::new(0).unwrap(),
        };
        let writers = if transform_first {
            [
                (&transform, ReadinessPhase::Execution),
                (&scalar, ReadinessPhase::Preparation),
            ]
        } else {
            [
                (&scalar, ReadinessPhase::Preparation),
                (&transform, ReadinessPhase::Execution),
            ]
        };
        for (writer, phase) in writers {
            proof.writer(writer, phase).unwrap();
        }
        proof
            .read(
                &PendingRead::Value(key.clone()),
                ReadinessPhase::Preparation,
            )
            .unwrap();
        assert!(
            proof
                .read(
                    &PendingRead::ModifierTransforms {
                        key: key.clone(),
                        initial: Box::new(key.clone()),
                    },
                    ReadinessPhase::Preparation
                )
                .is_err(),
            "late transform stream must remain late"
        );
        assert!(*proof.work < 1000);
    }
}

#[test]
fn symbolic_branches_and_ready_dependencies_cannot_hide_later_producers() {
    let f = fixture::generated_fixture();
    let inputs = fixture::compile_inputs(&f, fixture::target(30, "first"));
    let key = PlanValueKey::Stat {
        entity: ConcreteEntity::Actor(ActorKey::Player),
        stat: fixture::def("late"),
    };
    let mut work = 1000;
    let mut proof = ReadinessProof {
        stages: &inputs.stages,
        index: inputs.definitions.as_ref(),
        values: BTreeMap::from([(key.clone(), ReadinessPhase::Execution)]),
        contributions: BTreeMap::new(),
        transforms: BTreeMap::new(),
        effects: vec![ReadinessPhase::Structural, ReadinessPhase::Execution],
        work: &mut work,
    };
    assert!(
        proof
            .read(
                &PendingRead::Select {
                    decision: 0,
                    when_true: Box::new(PendingRead::Ready(ReadBinding::Constant(Some(
                        ParameterValue::Boolean(true)
                    )))),
                    when_false: Box::new(PendingRead::Value(key.clone())),
                },
                ReadinessPhase::Preparation
            )
            .is_err()
    );
    assert!(
        proof
            .read(
                &PendingRead::Ready(ReadBinding::Final {
                    effect: Some(1),
                    complete: true,
                }),
                ReadinessPhase::Preparation
            )
            .is_err()
    );
    assert!(
        proof
            .read(
                &PendingRead::ModifierTransforms {
                    key: key.clone(),
                    initial: Box::new(key),
                },
                ReadinessPhase::Preparation
            )
            .is_err(),
        "even an empty transform stream needs its initial scalar"
    );
}
