use bodymovin::helpers;

const EASING: &str = r#"
    "i": {"x": [0.833, 0.833, 0.833], "y": [0.833, 0.833, 0.833]},
    "o": {"x": [0.167, 0.167, 0.167], "y": [0.167, 0.167, 0.167]}
"#;

fn scale_keyframes(size_frame_tuple: Vec<(i16, i16)>, delay: i16) -> String {
    size_frame_tuple
        .iter()
        .map(|(size, frame)| {
            format!(
                "{{\"s\":[{}, {}, 100], \"t\": {}, {}}}",
                size,
                size,
                frame + delay,
                EASING
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

pub fn scale_transform(delay: Option<i16>) -> helpers::Transform {
    serde_json::from_str(&format!(
        r#"
            {{
                "p": {{
                    "k": [480, 480, 0]
                }},
                "s": {{
                    "k": [{}]
                }},
                "a": {{
                    "k": [240, -240]
                }}
            }}
        "#,
        scale_keyframes(
            vec![(100, 0), (120, 12), (90, 24), (100, 36)],
            delay.unwrap_or(0)
        )
    ))
    .unwrap()
}
