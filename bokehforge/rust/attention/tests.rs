use std::fs;
use std::path::PathBuf;

use ort::value::Tensor;
use serde_json::Value;

use super::{Backend, run_predictions, session_builder};

#[test]
fn inspection_schedules_report_the_first_model_failure() {
    fn output<T: Default>(error: Option<&str>) -> Result<T, String> {
        match error {
            Some(message) => Err(message.to_owned()),
            None => Ok(T::default()),
        }
    }

    for concurrent in [false, true] {
        for (reliqs, sam, tempsal, first) in [
            (Some("ReLIQS"), Some("SAM"), Some("TempSAL"), "ReLIQS"),
            (None, Some("SAM"), Some("TempSAL"), "SAM"),
            (None, None, Some("TempSAL"), "TempSAL"),
        ] {
            let predictions = run_predictions(
                || output(reliqs),
                || output(sam),
                || output(tempsal),
                concurrent,
            );
            assert_eq!(predictions.unwrap_err(), first);
        }
    }
}

#[test]
#[ignore = "requires a generated ONNX fixture and the ONNX Runtime library"]
fn configured_provider_executes_onnx_fixture() {
    let fixture = PathBuf::from(
        std::env::var_os("BOKEHFORGE_ORT_FIXTURE").expect("set the ONNX fixture directory"),
    );
    let (backend, provider) = if cfg!(target_os = "macos") {
        (Backend::CoreMl, "CoreMLExecutionProvider")
    } else {
        (Backend::Cpu, "CPUExecutionProvider")
    };
    let library = fs::read_to_string(fixture.join("runtime-path.txt")).unwrap();
    ort::init_from(library.trim()).unwrap().commit();
    let mut builder = session_builder(backend, false)
        .unwrap()
        .with_profiling(fixture.join("profile"))
        .unwrap();
    if cfg!(target_os = "macos") {
        builder = builder
            .with_config_entry("session.disable_cpu_ep_fallback", "1")
            .unwrap();
    }
    let mut session = builder
        .commit_from_file(fixture.join("model.onnx"))
        .unwrap();
    let input = Tensor::from_array(([1, 1, 2, 2], vec![-2.0_f32, -1.0, 0.0, 1.0])).unwrap();
    let outputs = session.run(ort::inputs!["input" => input]).unwrap();
    let (_, values) = outputs["output"].try_extract_tensor::<f32>().unwrap();
    assert_eq!(values, &[0.0, 0.0, 1.0, 2.0]);
    drop(outputs);

    let profile = fs::read_to_string(session.end_profiling().unwrap()).unwrap();
    let events: Vec<Value> = serde_json::from_str(&profile).unwrap();
    assert!(
        events
            .iter()
            .any(|event| event["args"]["provider"] == provider),
        "Expected an ONNX Runtime profile event for {provider}",
    );
}
