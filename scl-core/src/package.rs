//! 解析包名称，用于从 Maven 下载库

use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use nom::*;

fn between_colon(i: &str) -> IResult<&str, &str> {
    bytes::complete::take_till(|c| c == ':')(i)
}

pub fn parse_package_name(input: &str) -> IResult<&str, PackageName> {
    let (input, namespaces) = between_colon(input)?;
    let namespaces = namespaces.split('.').map(|s| s.to_string()).collect();
    let (input, _) = character::complete::char(':')(input)?;
    let (input, name) = between_colon(input)?;
    let (version, _) = character::complete::char(':')(input)?;
    Ok((
        "",
        PackageName {
            namespaces,
            name: name.into(),
            version: version.into(),
        },
    ))
}

#[derive(Debug, Clone, Default)]
pub struct PackageName {
    namespaces: Vec<String>,
    name: String,
    version: String,
}

impl PackageName {
    pub fn to_maven_jar_path(&self, path_or_url: &str) -> String {
        format!(
            "{}/{}/{}/{}/{}-{}.jar",
            path_or_url,
            self.namespaces.join("/"),
            self.name,
            self.version,
            self.name,
            self.version
        )
    }
}

impl Display for PackageName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            self.namespaces.join("."),
            self.name,
            self.version
        )
    }
}

impl From<&str> for PackageName {
    fn from(s: &str) -> Self {
        parse_package_name(s).unwrap().1
    }
}

impl FromStr for PackageName {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match parse_package_name(s) {
            Ok(r) => Ok(r.1),
            Err(_) => Err(()),
        }
    }
}

#[test]
fn parse_uri_test() {
    fn test_package(input: &str) {
        let result = parse_package_name(input);
        assert!(result.is_ok());
        let result = result.unwrap().1;
        assert_eq!(input, &format!("{}", result));
        println!("{}", result.to_maven_jar_path("https://maven.fabricmc.net"));
    }
    test_package("net.fabricmc:sponge-mixin:0.9.2+mixin.0.8.2");
    test_package("net.fabricmc:tiny-remapper:0.3.0.70");
    test_package("net.fabricmc:access-widener:1.0.0");
    test_package("net.fabricmc:fabric-loader-sat4j:2.3.5.4");
    test_package("com.google.jimfs:jimfs:1.2-fabric");
    test_package("org.ow2.asm:asm:9.1");
    test_package("org.ow2.asm:asm-analysis:9.1");
    test_package("org.ow2.asm:asm-commons:9.1");
    test_package("org.ow2.asm:asm-tree:9.1");
    test_package("org.ow2.asm:asm-util:9.1");
    test_package("net.fabricmc:intermediary:1.16.5");
    test_package("net.fabricmc:tiny-mappings-parser:0.2.2.14");
    test_package("net.fabricmc:fabric-loader:0.11.3");
}
