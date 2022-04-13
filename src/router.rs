use std::cell::RefCell;
use std::collections::HashSet;
use std::net::{Ipv4Addr};
use std::process::{Command};
use std::rc::Rc;

type Mac = String;

pub trait RouterInterface {
    fn get_online_mac_addresses(&self) -> Result<HashSet<Mac>, String>;
}

pub struct KeeneticRouterInterface {
    addr: Ipv4Addr,
    user: String,
    password: String,
}

impl KeeneticRouterInterface {
    pub fn new<A : Into<String>, B: Into<String>>(addr: Ipv4Addr, user: A, password: B) -> Self {
        Self { addr, user: user.into(), password: password.into() }
    }
}


impl RouterInterface for KeeneticRouterInterface {
    fn get_online_mac_addresses(&self) -> Result<HashSet<Mac>, String> {
        let stdout = Command::new("sshpass")
            .args(&["-p", &self.password, "ssh", &format!("{}@{}", self.user, self.addr), "show", "ip", "hotspot"])
            .output()
            .map_err(|a| a.to_string())?
            .stdout;

        let stdout = String::from_utf8(stdout).map_err(|a| a.to_string())?;

        type Node = Rc<RefCell<NodeInner>>;

        struct NodeInner {
            name: String,
            value: String,
            parent: Option<Node>,
            children: Vec<Node>,
        }
        impl NodeInner {
            pub fn new<A: Into<String>, B: Into<String>>(name: A, value: B, parent: Node) -> Node {
                Rc::new(RefCell::new(Self {
                    name: name.into(),
                    value: value.into(),
                    parent: Some(parent),
                    children: Default::default(),
                }))
            }
            pub fn root() -> Node {
                Rc::new(RefCell::new(Self {
                    name: "".to_owned(),
                    value: "".to_owned(),
                    parent: None,
                    children: Default::default(),
                }))
            }
        }

        fn child(parent: Node, name: &str, value: &str) -> Node {
            let child = NodeInner::new(name.trim(), value.trim(), parent.clone());
            parent.borrow_mut().children.push(child.clone());

            return child;
        }

        let root = NodeInner::root();

        let mut prev = root.clone();
        let mut node = root.clone();

        let mut category_stack = Vec::<i32>::new();
        category_stack.push(13);

        for line in stdout.lines() {
            if line.trim().is_empty() { continue; }
            let idx = line.find(":");
            if idx.is_none() { continue; }
            let idx = idx.unwrap();

            let name = &line[0..idx];
            let value = &line[idx + 1..];

            let idx = if name.contains(",") {
                (idx as i32 - name.len() as i32 + name.find(",").unwrap() as i32) as usize
            } else {
                idx
            };

            let idx = idx as i32;
            let delta = idx - category_stack.last().unwrap();
            if delta % 4 != 0 {
                return Err("Not divisible by 4".to_string());
            }

            if delta < 0 {
                for _ in 0..delta / -4 {
                    category_stack.pop();
                    let prev_node = node.borrow().parent.clone().unwrap().clone();
                    node = prev_node;
                }
                category_stack.push(idx);
                prev = child(node.clone(), name, value)
            } else if delta > 0 {
                if delta > 4 {
                    return Err("Cannot move forward for more than one step".to_owned());
                }
                category_stack.push(idx);
                node = prev.clone();
                prev = child(node.clone(), name, value);
            } else {
                prev = child(node.clone(), name, value);
            }
        }

        let mut result = HashSet::new();

        for child in &root.borrow().children {
            let child = child.borrow();
            if child.name != "host" {
                continue;
            }

            let mac = child.children.iter()
                .find(|a| a.borrow().name == "mac")
                .map(|a| a.borrow().value.clone())
                .ok_or("mac is not set")?;
            let active = child.children.iter()
                .find(|a| a.borrow().name == "active")
                .map(|a| a.borrow().value.clone())
                .ok_or("active is not set")?;


            if active == "yes" {
                result.insert(mac);
            }

        }

        return Ok(result);
    }
}
