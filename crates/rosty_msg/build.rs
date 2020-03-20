use quote::ToTokens;
use regex::RegexBuilder;
use rosty_msg_fmt::message_path::MessagePath;
use rosty_msg_fmt::msg::Msg;
use std::collections::{HashMap, HashSet, LinkedList};
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::{env, fs};

mod output_layout;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    rerun_if_env_changed("OUT_DIR");
    rerun_if_env_changed("CMAKE_PREFIX_PATH");
    rerun_if_env_changed("ROSRUST_MSG_PATH");

    let cmake_paths = env::var("CMAKE_PREFIX_PATH")
        .unwrap_or_default()
        .split(':')
        .filter_map(append_share_folder)
        .collect::<Vec<String>>();
    let cmake_alt_paths = env::var("CMAKE_PREFIX_PATH")
        .unwrap_or_default()
        .split(':')
        .filter_map(append_src_folder)
        .collect::<Vec<String>>();
    let extra_paths = env::var("ROSRUST_MSG_PATH")
        .unwrap_or_default()
        .split(':')
        .map(String::from)
        .collect::<Vec<String>>();
    let owned_paths = cmake_paths
        .iter()
        .chain(cmake_alt_paths.iter())
        .chain(extra_paths.iter())
        .collect::<Vec<_>>();
    let paths = owned_paths
        .iter()
        .map(|s| s.as_ref())
        .collect::<Vec<&str>>();
    for path in paths.iter() {
        rerun_if_folder_content_changed(Path::new(path));
    }

    //    let paths_owned = cmake_paths
    //        .iter()
    //        .chain(cmake_alt_paths.iter())
    //        .chain(extra_paths.iter())
    //        .flat_map(|v| find_all_package_groups(&Path::new(v)))
    //        .collect::<Vec<String>>();
    //    let paths = paths_owned
    //        .iter()
    //        .map(String::as_str)
    //        .collect::<Vec<&str>>();

    let messages = build_message_map(
        paths
            .iter()
            .flat_map(|path| find_all_messages_and_services(Path::new(path)))
            .collect(),
    );

    let layout = message_map_to_layout(&messages).unwrap();
    let tokens = layout.token_stream(&quote::quote! { crate:: });
    let contents = tokens.to_token_stream().to_string();

    //    let message_map = helpers::get_message_map(paths.as_slice(), &messages).unwrap();

    let file_name = format!("{}/{}", out_dir, "messages.rs");

    //    let package_names = messages
    //        .iter()
    //        .map(|(pkg, msg)| format!("{}/{}", pkg, msg))
    //        .collect::<Vec<String>>()
    //        .join(",");
    //    let package_tuples = messages
    //        .iter()
    //        .map(|(pkg, msg)| format!("(\"{}\",\"{}\")", pkg, msg))
    //        .collect::<Vec<String>>()
    //        .join(",");
    //
    let file_content = contents;

    fs::write(&file_name, &file_content).unwrap();

    Command::new("cargo")
        .arg("fmt")
        .arg("--")
        .arg(&file_name)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

fn rerun_if_file_changed(key: &str) {
    println!("cargo:rerun-if-changed={}", key);
}

fn rerun_if_env_changed(key: &str) {
    println!("cargo:rerun-if-env-changed={}", key);
}

pub fn rerun_if_folder_content_changed(folder: &Path) {
    if !folder.is_dir() {
        if folder.extension() == Some(OsStr::new("msg"))
            || folder.extension() == Some(OsStr::new("srv"))
        {
            if let Some(name) = folder.to_str() {
                rerun_if_file_changed(name);
            }
        }
        return;
    }
    if let Some(name) = folder.to_str() {
        rerun_if_file_changed(name);
    }
    if let Ok(children) = fs::read_dir(folder) {
        for child in children.filter_map(Result::ok) {
            rerun_if_folder_content_changed(&child.path());
        }
    }
}

fn find_all_messages_and_services(root: &Path) -> Vec<MessageCase> {
    if !root.is_dir() {
        return match identify_message_or_service(root) {
            Some(v) => vec![v],
            None => vec![],
        };
    }
    let mut items = vec![];
    if let Ok(children) = fs::read_dir(root) {
        for child in children.filter_map(|v| v.ok()) {
            items.append(&mut find_all_messages_and_services(&child.path()));
        }
    }
    items
}

fn identify_message_or_service(filename: &Path) -> Option<MessageCase> {
    let extension = filename.extension()?;
    let message = filename.file_stem()?;
    let parent = filename.parent()?;
    let grandparent = parent.parent()?;
    let package = grandparent.file_name()?;
    if Some(extension) != parent.file_name() {
        return None;
    }

    let message = MessagePath::new(package.to_str()?, message.to_str()?);

    let mut f = File::open(&filename).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).ok()?;

    match extension.to_str() {
        Some("msg") => {
            let msg = Msg::new(message, &contents).unwrap();
            Some(MessageCase::Message(msg))
        }
        Some("srv") => {
            let re = RegexBuilder::new("^---+$")
                .multi_line(true)
                .build()
                .unwrap();
            let (req, res) = match re.split(&contents).collect::<Vec<_>>().as_slice() {
                &[req] => (req, ""),
                &[req, res] => (req, res),
                &[] => panic!("Service {} does not have any content", message),
                v => panic!("Service {} is split into {} parts", message, v.len()),
            };
            let req = Msg::new(
                MessagePath::new(&message.package, format!("{}Req", &message.name)),
                req,
            )
            .unwrap();
            let res = Msg::new(
                MessagePath::new(&message.package, format!("{}Res", &message.name)),
                res,
            )
            .unwrap();
            Some(MessageCase::Service(message, req, res))
        }
        _ => None,
    }
}

fn append_share_folder(path: &str) -> Option<String> {
    Path::new(path).join("share").to_str().map(String::from)
}

fn append_src_folder(path: &str) -> Option<String> {
    Path::new(path)
        .join("..")
        .join("src")
        .to_str()
        .map(String::from)
}

#[derive(Debug)]
pub struct MessageMap {
    pub messages: HashMap<MessagePath, Msg>,
    pub services: HashSet<MessagePath>,
}

fn build_message_map(cases: Vec<MessageCase>) -> MessageMap {
    let mut messages = HashMap::new();
    let mut services = HashSet::new();
    for message in cases {
        match message {
            MessageCase::Message(msg) => {
                messages.insert(msg.path.clone(), msg);
            }
            MessageCase::Service(path, req, rec) => {
                messages.insert(req.path.clone(), req);
                messages.insert(rec.path.clone(), rec);
                services.insert(path);
            }
        }
    }

    for (_, msg) in messages.iter() {
        msg.path.validate().unwrap();
    }

    for msg in services.iter() {
        msg.validate().unwrap();
    }

    for (_, msg) in messages.iter() {
        for dependency in &msg.dependencies() {
            assert!(
                messages.contains_key(dependency),
                "missing message type {}, required by {}",
                dependency,
                msg.path
            );
        }
    }

    MessageMap { messages, services }
}

#[derive(Debug)]
enum MessageCase {
    Message(Msg),
    Service(MessagePath, Msg, Msg),
}

fn message_map_to_layout(
    message_map: &MessageMap,
) -> rosty_msg_fmt::error::Result<output_layout::Layout> {
    let mut output = output_layout::Layout {
        packages: Vec::new(),
    };
    let hashes = calculate_md5(&message_map)?;
    let packages = message_map
        .messages
        .iter()
        .map(|(message, _value)| message.package.clone())
        .chain(
            message_map
                .services
                .iter()
                .map(|message| message.package.clone()),
        )
        .collect::<HashSet<String>>();
    for package in packages {
        let mut package_data = output_layout::Package {
            name: package.clone(),
            messages: Vec::new(),
            services: Vec::new(),
        };
        let names = message_map
            .messages
            .iter()
            .filter(|&(message, _value)| message.package == package)
            .map(|(message, _value)| message.name.clone())
            .collect::<HashSet<String>>();
        for name in names {
            let key = MessagePath::new(&package, name);
            let message = message_map
                .messages
                .get(&key)
                .expect("Internal implementation contains mismatch in map keys")
                .clone();
            let md5sum = hashes
                .get(&key)
                .expect("Internal implementation contains mismatch in map keys")
                .clone();
            let msg_definition = generate_message_definition(&message_map.messages, &message)?;
            let msg_type = message.get_type();
            package_data.messages.push(output_layout::Message {
                message,
                msg_definition,
                msg_type,
                md5sum,
            });
        }
        let names = message_map
            .services
            .iter()
            .filter(|&message| &message.package == &package)
            .map(|message| message.name.clone())
            .collect::<HashSet<String>>();
        for name in names {
            let md5sum = hashes
                .get(&MessagePath::new(&package, &name))
                .expect("Internal implementation contains mismatch in map keys")
                .clone();
            let msg_type = format!("{}/{}", package, name);
            package_data.services.push(output_layout::Service {
                name,
                md5sum,
                msg_type,
            })
        }
        output.packages.push(package_data);
    }
    Ok(output)
}

pub fn calculate_md5(
    message_map: &MessageMap,
) -> rosty_msg_fmt::error::Result<HashMap<MessagePath, String>> {
    let mut representations = HashMap::<MessagePath, String>::new();
    let mut hashes = HashMap::<MessagePath, String>::new();
    while hashes.len() < message_map.messages.len() {
        let mut changed = false;
        for (key, value) in &message_map.messages {
            if hashes.contains_key(key) {
                continue;
            }
            if let Ok(answer) = value.get_md5_representation(&hashes) {
                hashes.insert(key.clone(), calculate_md5_from_representation(&answer));
                representations.insert(key.clone(), answer);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for message in &message_map.services {
        let key_req = MessagePath::new(&message.package, format!("{}Req", message.name));
        let key_res = MessagePath::new(&message.package, format!("{}Res", message.name));
        let req = match representations.get(&key_req) {
            Some(v) => v,
            None => error_chain::bail!("Message map does not contain all needed elements"),
        };
        let res = match representations.get(&key_res) {
            Some(v) => v,
            None => error_chain::bail!("Message map does not contain all needed elements"),
        };
        hashes.insert(
            message.clone(),
            calculate_md5_from_representation(&format!("{}{}", req, res)),
        );
    }
    if hashes.len() < message_map.messages.len() + message_map.services.len() {
        error_chain::bail!("Message map does not contain all needed elements");
    }
    Ok(hashes)
}

fn calculate_md5_from_representation(v: &str) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.input(v);
    hex::encode(hasher.result())
}

pub fn generate_message_definition<S: std::hash::BuildHasher>(
    message_map: &HashMap<MessagePath, Msg, S>,
    message: &Msg,
) -> rosty_msg_fmt::error::Result<String> {
    let mut handled_messages = HashSet::<MessagePath>::new();
    let mut result = message.source.clone();
    let mut pending = message
        .dependencies()
        .into_iter()
        .collect::<LinkedList<_>>();
    while let Some(value) = pending.pop_front() {
        if handled_messages.contains(&value) {
            continue;
        }
        handled_messages.insert(value.clone());
        result += "\n\n========================================";
        result += "========================================";
        result += &format!("\nMSG: {}\n", value);
        let message = match message_map.get(&value) {
            Some(msg) => msg,
            None => error_chain::bail!("Message map does not contain all needed elements"),
        };
        for dependency in message.dependencies() {
            pending.push_back(dependency);
        }
        result += &message.source;
    }
    result += "\n";
    Ok(result)
}
