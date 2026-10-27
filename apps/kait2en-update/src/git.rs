// SPDX-License-Identifier: GPL-3.0-or-later
//! Read only access to the system KaiT2en checkout.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub const BRANCH: &str = "main";
pub const REMOTE_REF: &str = "origin/main";

/// One commit between the installed state and GitHub.
pub struct Commit {
    pub short: String,
    pub date: String,
    pub subject: String,
}

pub struct Repository {
    path: PathBuf,
}

impl Repository {
    /// The checkout kait2en-install keeps, KAIT2EN_REPOSITORY overrides it.
    pub fn system() -> Self {
        let path = std::env::var_os("KAIT2EN_REPOSITORY")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr/local/src/KaiT2en-Fedora"));
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn git(&self, args: &[&str]) -> Result<String, String> {
        let safe = format!("safe.directory={}", self.path.display());
        let output = Command::new("git")
            .arg("-c")
            .arg(&safe)
            .arg("-C")
            .arg(&self.path)
            .args(args)
            .output()
            .map_err(|error| format!("Could not run git: {error}"))?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
        }
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(if message.is_empty() { format!("git {} failed", args.join(" ")) } else { message })
    }

    /// Fetches GitHub as the user, who owns the checkout.
    pub fn fetch(&self) -> Result<(), String> {
        self.git(&["fetch", "--quiet", "origin", BRANCH]).map(|_| ())
    }

    pub fn resolve(&self, rev: &str) -> Result<String, String> {
        let spec = format!("{rev}^{{commit}}");
        Ok(self.git(&["rev-parse", "--verify", "--quiet", &spec])?.trim().to_owned())
    }

    pub fn has_commit(&self, rev: &str) -> bool {
        self.resolve(rev).is_ok()
    }

    pub fn commit_date(&self, rev: &str) -> String {
        self.git(&["show", "-s", "--format=%cs", rev])
            .map(|date| date.trim().to_owned())
            .unwrap_or_default()
    }

    pub fn changed_files(&self, base: &str, target: &str) -> Result<Vec<String>, String> {
        Ok(self
            .git(&["diff", "--name-only", "--no-renames", base, target])?
            .lines()
            .map(str::to_owned)
            .collect())
    }

    pub fn commits(&self, base: &str, target: &str) -> Result<Vec<Commit>, String> {
        let range = format!("{base}..{target}");
        Ok(self
            .git(&["log", "--format=%h%x09%cs%x09%s", &range])?
            .lines()
            .filter_map(|line| {
                let mut fields = line.splitn(3, '\t');
                Some(Commit {
                    short: fields.next()?.to_owned(),
                    date: fields.next()?.to_owned(),
                    subject: fields.next()?.to_owned(),
                })
            })
            .collect())
    }

    /// A file as it is in a commit, None when it does not exist there.
    pub fn show(&self, rev: &str, path: &str) -> Option<String> {
        self.git(&["show", &format!("{rev}:{path}")]).ok()
    }
}
