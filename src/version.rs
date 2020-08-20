pub enum VersionInfo {
    num(String),
    filename(String, String),
}

#[derive(Debug, PartialOrd, PartialEq)]
///Struct contains version information, the string field
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
    rest: Vec<u32>,
    pub string: String,
}

impl Version {
    ///version_str - string that contains version number
    ///format - example of system kernel versioning. If version str is just the versioning, 
    pub fn new(version_str: String, format: Option<&str>) -> Self {
        let mut version_str_split = version_str.split(|x| (x == '.') || (x == '-') || (x == '_'));
        let mut nums: Vec<u32> = vec![];
        while let Some(letter) = version_str_split.next() {
            let converted = letter.parse::<u32>();
            if let Ok(num) = converted {
                nums.push(num);
            }
        }
        let mut nums_iter = nums.into_iter();
        let string: String; 

        match format {
            Some(format) => {
                let mut format_split = format.split("%v");
                let ignore_pre = format_split.next().unwrap().len();
                let ignore_post = format_split.next().unwrap().len();
                string = String::from(&version_str[ignore_pre..(ignore_post + version_str.len())]);
            },
            None => {
                string = version_str;
            }
        }

        Version {
            major: nums_iter.next().expect("Error with versioning"),
            minor: nums_iter.next().expect("Error with verisoning"),
            patch: nums_iter.next().expect("Error with versioning"),
            rest: nums_iter.collect(),
            string,
        }
    }
}
