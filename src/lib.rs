use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

#[derive(Debug)]
pub struct TodoFile {
    pub path: PathBuf,
    pub tasks: Vec<Task>,
}

impl TodoFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();
        let tasks = if !path.exists() {
            fs::write(&path, "")?;
            Vec::new()
        } else {
            let contents = fs::read_to_string(&path)?;
            contents
                .lines()
                .filter(|line| !line.is_empty())
                .map(|line| line.parse())
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(TodoFile { path, tasks })
    }

    pub fn save(&self) -> Result<(), Error> {
        let mut writer = fs::File::create(&self.path)?;
        for task in &self.tasks {
            writer.write(task.to_string().as_bytes())?;
            writer.write(b"\n")?;
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct Task {
    pub summary: String,
    pub completed: bool,
}

impl FromStr for Task {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.starts_with("x") {
            Ok(Task {
                summary: s[1..].trim().to_string(),
                completed: true,
            })
        } else {
            Ok(Task {
                summary: s.to_string(),
                completed: false,
            })
        }
    }
}

impl ToString for Task {
    fn to_string(&self) -> String {
        if self.completed {
            format!("x {}", self.summary)
        } else {
            format!("  {}", self.summary)
        }
    }
}
