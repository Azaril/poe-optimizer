//! Canonical failure selection for an unordered contribution reduction. This
//! order chooses diagnostic provenance only, never a Boolean or numeric result.
use super::*;
use std::cmp::Ordering;

fn gap(reason: PlanGapReason) -> u8 {
    match reason {
        PlanGapReason::MissingSupportSources => 20,
        PlanGapReason::UnmappedSupportSources => 21,
        PlanGapReason::PartialExistingActorRules => 0,
        PlanGapReason::PartialEffectApplications => 1,
        PlanGapReason::SchemaUnresolved => 2,
        PlanGapReason::MissingPrograms => 3,
        PlanGapReason::PartialPrograms => 4,
        PlanGapReason::PartialReceivers => 5,
        PlanGapReason::PartialDeclarations => 6,
        PlanGapReason::UnresolvedTopology => 7,
        PlanGapReason::UnsupportedContext => 8,
        PlanGapReason::UnsupportedRelation => 9,
        PlanGapReason::MissingRouting => 10,
        PlanGapReason::PartialRouting => 11,
        PlanGapReason::MissingInput => 12,
        PlanGapReason::MissingProducer => 13,
        PlanGapReason::MissingMetricBinding => 14,
        PlanGapReason::IncompleteContributors => 15,
        PlanGapReason::UpstreamUnavailable => 16,
        PlanGapReason::UnresolvedActivation => 17,
        PlanGapReason::ConflictingContributors => 18,
        PlanGapReason::AbsentSelection => 19,
    }
}
fn numerical(reason: NumericalFailure) -> u8 {
    match reason {
        NumericalFailure::DivisionByZero => 0,
        NumericalFailure::NonFinite => 1,
        NumericalFailure::IntegerOverflow => 2,
    }
}
fn value_kind(value: &ParameterValue) -> u8 {
    match value {
        ParameterValue::Boolean(_) => 0,
        ParameterValue::Integer(_) => 1,
        ParameterValue::Quantity(_) => 2,
        ParameterValue::Option(_) => 3,
    }
}
fn value(a: &ParameterValue, b: &ParameterValue) -> Ordering {
    match (a, b) {
        (ParameterValue::Boolean(a), ParameterValue::Boolean(b)) => a.cmp(b),
        (ParameterValue::Integer(a), ParameterValue::Integer(b)) => a.cmp(b),
        (ParameterValue::Quantity(a), ParameterValue::Quantity(b)) => a
            .unit()
            .cmp(b.unit())
            .then_with(|| a.value().total_cmp(&b.value())),
        (ParameterValue::Option(a), ParameterValue::Option(b)) => a.cmp(b),
        _ => value_kind(a).cmp(&value_kind(b)),
    }
}
fn kind(value: &EffectValue) -> u8 {
    match value {
        EffectValue::Unresolved { .. } => 0,
        EffectValue::UnsupportedValue { .. } => 1,
        EffectValue::UnsupportedDomain { .. } => 2,
        EffectValue::NumericalError { .. } => 3,
        EffectValue::Known { .. } | EffectValue::Inactive => {
            unreachable!("only failures are compared")
        }
    }
}
fn range(a: &NumericDomain, b: &NumericDomain) -> Ordering {
    match (a, b) {
        (NumericDomain::Integer(a), NumericDomain::Integer(b)) => {
            (a.minimum, a.maximum).cmp(&(b.minimum, b.maximum))
        }
        (NumericDomain::Quantity(a), NumericDomain::Quantity(b)) => a
            .minimum
            .unit()
            .cmp(b.minimum.unit())
            .then_with(|| a.minimum.value().total_cmp(&b.minimum.value()))
            .then_with(|| a.maximum.unit().cmp(b.maximum.unit()))
            .then_with(|| a.maximum.value().total_cmp(&b.maximum.value())),
        (NumericDomain::Integer(_), NumericDomain::Quantity(_)) => Ordering::Less,
        (NumericDomain::Quantity(_), NumericDomain::Integer(_)) => Ordering::Greater,
    }
}
fn domain(a: &UnsupportedDomainCause, b: &UnsupportedDomainCause) -> Ordering {
    match (a, b) {
        (
            UnsupportedDomainCause::IntegerTable {
                table: at,
                key: ak,
                minimum: al,
                maximum: ah,
            },
            UnsupportedDomainCause::IntegerTable {
                table: bt,
                key: bk,
                minimum: bl,
                maximum: bh,
            },
        ) => (at, ak, al, ah).cmp(&(bt, bk, bl, bh)),
        (
            UnsupportedDomainCause::NumericRange {
                value: av,
                domain: ad,
            },
            UnsupportedDomainCause::NumericRange {
                value: bv,
                domain: bd,
            },
        ) => range(ad, bd).then_with(|| value(av, bv)),
        (
            UnsupportedDomainCause::IntegerTable { .. },
            UnsupportedDomainCause::NumericRange { .. },
        ) => Ordering::Less,
        (
            UnsupportedDomainCause::NumericRange { .. },
            UnsupportedDomainCause::IntegerTable { .. },
        ) => Ordering::Greater,
    }
}
pub(super) fn compare(a: &EffectValue, b: &EffectValue) -> Ordering {
    match (a, b) {
        (
            EffectValue::Unresolved {
                reason: a,
                read: ar,
            },
            EffectValue::Unresolved {
                reason: b,
                read: br,
            },
        ) => gap(*a).cmp(&gap(*b)).then_with(|| ar.cmp(br)),
        (
            EffectValue::UnsupportedValue { value: a },
            EffectValue::UnsupportedValue { value: b },
        ) => value(a, b),
        (
            EffectValue::UnsupportedDomain {
                node: an,
                domain: ad,
            },
            EffectValue::UnsupportedDomain {
                node: bn,
                domain: bd,
            },
        ) => an.cmp(bn).then_with(|| domain(ad, bd)),
        (
            EffectValue::NumericalError {
                node: an,
                reason: ar,
            },
            EffectValue::NumericalError {
                node: bn,
                reason: br,
            },
        ) => numerical(*ar).cmp(&numerical(*br)).then_with(|| an.cmp(bn)),
        _ => kind(a).cmp(&kind(b)),
    }
}
