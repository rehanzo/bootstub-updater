#[derive(Debug, PartialOrd, PartialEq)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
    rest: Vec<u32>,
    pub string: String,
}

impl Version {
    pub fn new(version_str: String, format: String, full_str: bool) -> Self {
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

        if full_str {
            let mut format_split = format.split("%v");
            let ignore_pre = format_split.next().unwrap().len();
            let ignore_post = format_split.next().unwrap().len();
            string = String::from(&version_str[ignore_pre..(ignore_post + version_str.len())]);
        }
        else {
            string = version_str;
        }


        Version {
            major: nums_iter.next().unwrap(),
            minor: nums_iter.next().unwrap(),
            patch: nums_iter.next().unwrap(),
            rest: nums_iter.collect(),
            string,
        }
    }
}
