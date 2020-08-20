use std::process::Command;
#[derive(Debug, PartialOrd, PartialEq)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
    rest: Vec<u32>,
}

impl Version {
    pub fn new(version_str: String) -> Self {
        let mut version_str_split = version_str.split(|x| (x == '.') || (x == '-') || (x == '_'));
        let mut nums: Vec<u32> = vec![];
        while let Some(letter) = version_str_split.next() {
            let converted = letter.parse::<u32>();
            if let Ok(num) = converted {
                nums.push(num);
            }
        }
        let mut nums_iter = nums.into_iter();

        Version {
            major: nums_iter.next().unwrap(),
            minor: nums_iter.next().unwrap(),
            patch: nums_iter.next().unwrap(),
            rest: nums_iter.collect(),
        }
    }
}
