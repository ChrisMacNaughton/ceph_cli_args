#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate yaml_rust;

use yaml_rust::YamlLoader;

use std::fs::File;
use std::io::prelude::*;
use log::LogLevel;

#[cfg(test)]
mod tests{
    use log::LogLevel;
    #[test]
    fn test_parse_file() {
        let file = r#"
outputs:
  - stdout
  - influx
influx:
  host: 127.0.0.1
  port: 8086
  user: root
  password: root
"#;
        let args = super::parse(file, LogLevel::Info).unwrap();

        assert_eq!(args.outputs, vec!["stdout", "influx"]);
        assert_eq!(args.influx.unwrap().port, "8086");
    }
}

#[derive(Clone,Debug)]
pub struct Args {
    pub carbon: Option<Carbon>,
    pub influx: Option<Influx>,
    pub elasticsearch: Option<String>,
    pub stdout: Option<String>,
    pub outputs: Vec<String>,
    pub config_path: String,
    pub log_level: log::LogLevel,
}

struct CliArgs {
    log_level: log::LogLevel,
    config_file: String,
}

impl Args {
    fn clean() -> Args {
        Args {
            carbon: None,
            influx: None,
            elasticsearch: None,
            stdout: None,
            outputs: Vec::new(),
            config_path: "".to_string(),
            log_level: LogLevel::Info,
        }
    }
    fn with_log_level(log_level: LogLevel) -> Args {
        Args {
            carbon: None,
            influx: None,
            elasticsearch: None,
            stdout: None,
            outputs: Vec::new(),
            config_path: "".to_string(),
            log_level: log_level,
        }
    }
}

#[derive(Clone,Debug)]
pub struct Influx {
    pub user: String,
    pub password: String,
    pub host: String,
    pub port: String
}

#[derive(Clone,Debug)]
pub struct Carbon {
    pub host: String,
    pub port: String,
    pub root_key: String,
}

pub fn get_args() -> Args {
    let cli_args = get_cli_args();
    let yaml_text = match read_from_file(cli_args.config_file.as_ref()) {
        Ok(yaml) => yaml,
        Err(_) => "".to_string(),
    };
    let args = parse(yaml_text.as_ref(), cli_args.log_level).unwrap_or(Args::clean());
    args
}

fn parse(args_string: &str, log_level: LogLevel) -> Result<Args, String> {
    let config_path = "/etc/default/decode_ceph.yaml";

    // Remove this hack when the new version of yaml_rust releases to get the real
    // error msg
    let docs = match YamlLoader::load_from_str(&args_string) {
        Ok(data) => data,
        Err(_) => {
            // error!("Unable to load yaml data from config file");
            return Err("cannot load data from yaml".to_string());
        }
    };
    if docs.len() == 0 {
        return Ok(Args::with_log_level(log_level));
    }
    let doc = &docs[0];

    let elasticsearch = match doc["elasticsearch"].as_str() {
        Some(o) => Some(format!("http://{}/ceph/operations", o)),
        None => None,
    };
    let stdout = match doc["stdout"].as_str() {
        Some(o) => Some(o.to_string()),
        None => None
    };
    let influx_doc = doc["influx"].clone();
    let influx_host = influx_doc["host"].as_str().unwrap_or("127.0.0.1");
    let influx_port = influx_doc["port"].as_str().unwrap_or("8086");
    let influx_password = influx_doc["password"].as_str().unwrap_or("root");
    let influx_user = influx_doc["user"].as_str().unwrap_or("root");
    let influx = Influx {
        host: influx_host.to_string(),
        port: influx_port.to_string(),
        password: influx_password.to_string(),
        user: influx_user.to_string(),
    };

    let carbon_doc = doc["carbon"].clone();

    let carbon_host = match carbon_doc["host"].as_str() {
        Some(h) => Some(h),
        None => None,
    };

    let carbon_port = carbon_doc["port"].as_str().unwrap_or("2003");
    let root_key = carbon_doc["root_key"].as_str().unwrap_or("ceph");

    let carbon = match carbon_host {
        Some(h) => Some(Carbon {
            host: h.to_string(),
            port: carbon_port.to_string(),
            root_key: root_key.to_string(),
        }),
        None => None,
    };

    let outputs: Vec<String> = match doc["outputs"].as_vec() {
        Some(o) => {
            o.iter()
             .map(|x| {
                 match x.as_str() {
                     Some(o) => o.to_string(),
                     None => "".to_string(),
                 }
             })
             .collect()
        }
        None => Vec::new(),
    };

    Ok(Args {
        carbon: carbon,
        elasticsearch: elasticsearch,
        stdout: stdout,
        influx: Some(influx),
        outputs: outputs,
        log_level: log_level,
        config_path: config_path.to_string(),
    })
}

fn read_from_file(config_path: &str) -> Result<String, String> {
    let mut f = try!(File::open(config_path).map_err(|e| e.to_string()));

    let mut s = String::new();
    try!(f.read_to_string(&mut s).map_err(|e| e.to_string()));
    Ok(s.to_string())
}

fn get_cli_args() -> CliArgs {
    let matches = clap_app!(args =>
        (@arg CONFIG: -c --config +takes_value "Path to config file")
        (@arg debug: -d ... "Sets the level of debugging information")
    ).get_matches();

    let log_level = match matches.occurrences_of("debug") {
        0 => log::LogLevel::Warn,
        1 => log::LogLevel::Info,
        2 => log::LogLevel::Debug,
        3 | _ => log::LogLevel::Trace,
    };
    CliArgs {
        log_level: log_level,
        config_file: matches.value_of("CONFIG").unwrap_or("/etc/default/decode_ceph.yaml").to_string()
    }
}