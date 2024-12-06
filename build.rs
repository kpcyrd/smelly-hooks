use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

const TEST_DATA_PATH: &str = "test_data";
const TESTS_OUTPUT: &str = "generated_tests.rs";

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed={TEST_DATA_PATH}");

    let mut entries = fs::read_dir(TEST_DATA_PATH)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();

    let path = PathBuf::from(&env::var("OUT_DIR").unwrap()).join(TESTS_OUTPUT);
    let mut file = File::create(path)?;

    for entry in entries {
        let Some(filename) = entry.file_name() else {
            continue;
        };
        let Some(filename) = filename.to_str() else {
            continue;
        };
        let Some(pkg) = filename.strip_suffix(".install") else {
            continue;
        };
        let function = pkg
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>();

        println!("entry={entry:?}");
        file.write_all(
            &format!(
                r#"
    #[test]
    fn parse_{function}() {{
        let harness = harness();
        let script = std::fs::read_to_string({entry:?}).unwrap();
        match (validate(&script), harness.get({pkg:?})) {{
            (Ok(findings), Some(Entry::Findings(expected))) => if findings != *expected {{
                panic!("Wrong expected findings {pkg}: expected={{expected:?}}, findings={{findings:?}}");
            }},
            (Ok(_findings), Some(Entry::Error(expected))) => panic!("Missing expected error {pkg}: {{expected:?}}"),
            (Ok(findings), None) => if !findings.is_empty() {{
                panic!("Unexpected findings {pkg}: {{findings:?}}");
            }},
            (Err(err), Some(Entry::Findings(_expected))) => panic!("Unexpected error {pkg}: {{:?}}", err.to_string()),
            (Err(err), Some(Entry::Error(expected))) => {{
                let err = err.to_string();
                if err != *expected {{
                    panic!("Wrong error {pkg}: expected={{expected:?}}, err={{err:?}}");
                }}
            }},
            (Err(err), None) => panic!("Unexpected error {pkg}: {{:?}}", err.to_string()),
        }}
    }}

"#
            )
            .into_bytes(),
        )?;
    }

    Ok(())
}
