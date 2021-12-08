use std::collections::HashMap;

use subprocess::{Exec, Redirection};

pub fn run_script(cwd: &str, script: &str, variables: &HashMap<&str, &String>) -> bool {
    let run = Exec::cmd("bash")
        .cwd(cwd)
        .env_clear()
        .stdin(create_script(script, variables).as_str())
        .stderr(Redirection::Merge)
        .capture()
        .unwrap();

    run.success()
}

fn create_script(script: &str, variables: &HashMap<&str, &String>) -> String {
    let mut bash = "set -ex\n\n".to_string();

    for (key, &val) in variables.iter() {
        bash += &format!("{}='{}'\n", key, val);
    }

    bash += &format!("\n{}\nexit 0\n", script);
    bash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_output() {
        let script = "echo $somevar\n";
        let somevar = String::from("testing");
        let mut vars = HashMap::new();
        vars.insert("somevar", &somevar);
        assert_eq!(create_script(&script, &vars),
        "set -ex\n\nsomevar='testing'\n\necho $somevar\n\nexit 0\n");
    }
}
