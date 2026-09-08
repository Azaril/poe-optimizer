//! Numerical transport must preserve the bits used by data identities and evidence.
use poe_optimizer_core::metrics::MeasurementValue;

#[test]
fn finite_measurements_preserve_bits_through_json_text_and_values() {
    // The small decimal and large quotient previously drifted by one ULP with
    // serde_json's default best-effort float parser.
    let values = [
        0.0,
        -0.0,
        0.033,
        1.234_567_890_123_456_7,
        1.2e-307,
        -1.2e-307,
        25.0 / f64::from_bits(4_u64 << 52),
        -(25.0 / f64::from_bits(4_u64 << 52)),
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::from_bits(0x000f_ffff_ffff_ffff),
        f64::MAX,
        -f64::MAX,
    ];
    for value in values {
        let measurement = MeasurementValue::Finite { value };
        let bytes = serde_json::to_vec(&measurement).unwrap();
        let direct: MeasurementValue = serde_json::from_slice(&bytes).unwrap();
        let expected = serde_json::to_value(&measurement).unwrap();
        let decoded: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(expected, decoded, "evidence value {value:?}");
        let via_value: MeasurementValue = serde_json::from_value(decoded).unwrap();
        for restored in [direct, via_value] {
            assert_eq!(
                restored.finite().unwrap().to_bits(),
                value.to_bits(),
                "measurement {value:?}"
            );
        }
    }
}
