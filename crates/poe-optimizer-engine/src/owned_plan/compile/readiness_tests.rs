//! Focused dependency-channel tests complement the public graph witness.
use super::*;
use crate::owned_plan::compile::support_fixture as fixture;

#[test]
fn item_preparation_keeps_occurrence_identity_and_rejects_late_dependencies() {
    let f = fixture::generated_fixture();
    let inputs = fixture::compile_inputs(&f, fixture::target(30, "first"));
    let first = fixture::occurrence(51);
    let second = fixture::occurrence(52);
    for entities in [
        [
            ConcreteEntity::EquipmentUse(first),
            ConcreteEntity::EquipmentUse(second),
        ],
        [
            ConcreteEntity::Modifier(root(ProviderRoot::EquipmentUse(first))),
            ConcreteEntity::Modifier(root(ProviderRoot::EquipmentUse(second))),
        ],
    ] {
        let [early, late] = entities.map(|entity| PlanValueKey::Stat {
            entity,
            stat: fixture::def("same-local-item-fact"),
        });
        let mut work = 1000;
        let mut proof = ReadinessProof {
            stages: &inputs.stages,
            index: inputs.definitions.as_ref(),
            values: BTreeMap::new(),
            contributions: BTreeMap::new(),
            transforms: BTreeMap::new(),
            effects: vec![ReadinessPhase::Structural],
            work: &mut work,
        };
        for (key, phase) in [
            (early.clone(), ReadinessPhase::Structural),
            (late.clone(), ReadinessPhase::Execution),
        ] {
            proof
                .writer(&BoundEffectTarget::Value { key }, phase)
                .unwrap();
        }
        proof
            .read(
                &PendingRead::Value(early.clone()),
                ReadinessPhase::Preparation,
            )
            .unwrap();
        assert!(
            proof
                .read(
                    &PendingRead::Value(late.clone()),
                    ReadinessPhase::Preparation
                )
                .is_err()
        );
        assert!(
            proof
                .read(
                    &PendingRead::Select {
                        decision: 0,
                        when_true: Box::new(PendingRead::Value(early.clone())),
                        when_false: Box::new(PendingRead::Required(Box::new(PendingRead::Value(
                            late
                        )))),
                    },
                    ReadinessPhase::Preparation
                )
                .is_err(),
            "a candidate branch cannot hide a late item producer"
        );
        assert!(
            proof
                .writer(
                    &BoundEffectTarget::Value { key: early },
                    ReadinessPhase::Structural
                )
                .is_err(),
            "the same occurrence still rejects competing early writers"
        );
    }
}

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
