extern crate amqp;
extern crate env_logger;

use serde_json;

use hubcaps;
use ofborg::message::buildresult::BuildResult;
use ofborg::worker;
use amqp::protocol::basic::{Deliver, BasicProperties};


pub struct GitHubCommentPoster {
    github: hubcaps::Github,
}

impl GitHubCommentPoster {
    pub fn new(github: hubcaps::Github) -> GitHubCommentPoster {
        return GitHubCommentPoster { github: github };
    }
}

impl worker::SimpleWorker for GitHubCommentPoster {
    type J = BuildResult;

    fn msg_to_job(
        &mut self,
        _: &Deliver,
        _: &BasicProperties,
        body: &Vec<u8>,
    ) -> Result<Self::J, String> {
        return match serde_json::from_slice(body) {
            Ok(e) => Ok(e),
            Err(e) => {
                Err(format!(
                    "Failed to deserialize BuildResult: {:?}, err: {:}",
                    String::from_utf8_lossy(&body.clone()),
                    e
                ))
            }
        };
    }

    fn consumer(&mut self, job: &BuildResult) -> worker::Actions {
        let comment = hubcaps::comments::CommentOptions { body: result_to_comment(&job) };

        let comment_attempt = self.github
            .repo(job.repo.owner.clone(), job.repo.name.clone())
            .pulls()
            .get(job.pr.number)
            .comments()
            .create(&comment);

        match comment_attempt {
            Ok(comment) => {
                info!(
                "Successfully sent {:?} to {}",
                comment,
                job.pr.number,
            )
            }
            Err(err) => {
                info!(
                "Failed to send comment {:?} to {}",
                err,
                job.pr.number,
            )
            }
        }

        return vec![worker::Action::Ack];
    }
}

fn result_to_comment(result: &BuildResult) -> String {
    let mut reply: Vec<String> = vec![];

    reply.push(format!(
        "{} on {} [(full log)](https://logs.nix.ci/?key={}/{}.{}&attempt_id={})",
        (match result.success {
             true => "Success",
             false => "Failure",
         }),
        result.system,
        &result.repo.owner.to_lowercase(),
        &result.repo.name.to_lowercase(),
        result.pr.number,
        (match result.attempt_id {
             Some(ref attempt_id) => &attempt_id,
             None => "none",
         })
    ));
    reply.push("".to_owned());
    reply.push(
        "<details><summary>Partial log (click to expand)</summary><p>".to_owned(),
    );
    reply.push("".to_owned());
    reply.push("```".to_owned());
    reply.extend(result.output.clone());
    reply.push("```".to_owned());
    reply.push("</p></details>".to_owned());
    reply.push("".to_owned());
    reply.push("".to_owned());

    reply.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use message::{Pr, Repo};

    #[test]
    pub fn test_passing_build() {
        let result = BuildResult {
            repo: Repo {
                clone_url: "https://github.com/nixos/nixpkgs.git".to_owned(),
                full_name: "NixOS/nixpkgs".to_owned(),
                owner: "NixOS".to_owned(),
                name: "nixpkgs".to_owned(),
            },
            pr: Pr {
                head_sha: "abc123".to_owned(),
                number: 2345,
                target_branch: Some("master".to_owned()),
            },
            output: vec![
                "make[2]: Entering directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1/readline'".to_owned(),
                "make[2]: Nothing to be done for 'install'.".to_owned(),
                "make[2]: Leaving directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1/readline'".to_owned(),
                "make[1]: Nothing to be done for 'install-target'.".to_owned(),
                "make[1]: Leaving directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1'".to_owned(),
                "removed '/nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1/share/info/bfd.info'".to_owned(),
                "post-installation fixup".to_owned(),
                "strip is /nix/store/5a88zk3jgimdmzg8rfhvm93kxib3njf9-cctools-binutils-darwin/bin/strip".to_owned(),
                "patching script interpreter paths in /nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1".to_owned(),
                "/nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1".to_owned(),
            ],
            attempt_id: Some("neatattemptid".to_owned()),
            system: "x86_64-linux".to_owned(),
            success: true,
        };

        assert_eq!(
            &result_to_comment(&result),
            "Success on x86_64-linux [(full log)](https://logs.nix.ci/?key=nixos/nixpkgs.2345&attempt_id=neatattemptid)

<details><summary>Partial log (click to expand)</summary><p>

```
make[2]: Entering directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1/readline'
make[2]: Nothing to be done for 'install'.
make[2]: Leaving directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1/readline'
make[1]: Nothing to be done for 'install-target'.
make[1]: Leaving directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1'
removed '/nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1/share/info/bfd.info'
post-installation fixup
strip is /nix/store/5a88zk3jgimdmzg8rfhvm93kxib3njf9-cctools-binutils-darwin/bin/strip
patching script interpreter paths in /nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1
/nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1
```
</p></details>

"
        );
    }

    #[test]
    pub fn test_failing_build() {
        let result = BuildResult {
            repo: Repo {
                clone_url: "https://github.com/nixos/nixpkgs.git".to_owned(),
                full_name: "NixOS/nixpkgs".to_owned(),
                owner: "NixOS".to_owned(),
                name: "nixpkgs".to_owned(),
            },
            pr: Pr {
                head_sha: "abc123".to_owned(),
                number: 2345,
                target_branch: Some("master".to_owned()),
            },
            output: vec![
                "make[2]: Entering directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1/readline'".to_owned(),
                "make[2]: Nothing to be done for 'install'.".to_owned(),
                "make[2]: Leaving directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1/readline'".to_owned(),
                "make[1]: Nothing to be done for 'install-target'.".to_owned(),
                "make[1]: Leaving directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1'".to_owned(),
                "removed '/nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1/share/info/bfd.info'".to_owned(),
                "post-installation fixup".to_owned(),
                "strip is /nix/store/5a88zk3jgimdmzg8rfhvm93kxib3njf9-cctools-binutils-darwin/bin/strip".to_owned(),
                "patching script interpreter paths in /nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1".to_owned(),
                "/nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1".to_owned(),
            ],
            attempt_id: Some("neatattemptid".to_owned()),
            system: "x86_64-linux".to_owned(),
            success: false,
        };

        assert_eq!(
            &result_to_comment(&result),
            "Failure on x86_64-linux [(full log)](https://logs.nix.ci/?key=nixos/nixpkgs.2345&attempt_id=neatattemptid)

<details><summary>Partial log (click to expand)</summary><p>

```
make[2]: Entering directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1/readline'
make[2]: Nothing to be done for 'install'.
make[2]: Leaving directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1/readline'
make[1]: Nothing to be done for 'install-target'.
make[1]: Leaving directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1'
removed '/nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1/share/info/bfd.info'
post-installation fixup
strip is /nix/store/5a88zk3jgimdmzg8rfhvm93kxib3njf9-cctools-binutils-darwin/bin/strip
patching script interpreter paths in /nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1
/nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1
```
</p></details>

"
        );
    }

    #[test]
    pub fn test_failing_build_no_attempt_id() {
        let result = BuildResult {
            repo: Repo {
                clone_url: "https://github.com/nixos/nixpkgs.git".to_owned(),
                full_name: "NixOS/nixpkgs".to_owned(),
                owner: "NixOS".to_owned(),
                name: "nixpkgs".to_owned(),
            },
            pr: Pr {
                head_sha: "abc123".to_owned(),
                number: 2345,
                target_branch: Some("master".to_owned()),
            },
            output: vec![
                "make[2]: Entering directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1/readline'".to_owned(),
                "make[2]: Nothing to be done for 'install'.".to_owned(),
                "make[2]: Leaving directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1/readline'".to_owned(),
                "make[1]: Nothing to be done for 'install-target'.".to_owned(),
                "make[1]: Leaving directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1'".to_owned(),
                "removed '/nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1/share/info/bfd.info'".to_owned(),
                "post-installation fixup".to_owned(),
                "strip is /nix/store/5a88zk3jgimdmzg8rfhvm93kxib3njf9-cctools-binutils-darwin/bin/strip".to_owned(),
                "patching script interpreter paths in /nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1".to_owned(),
                "/nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1".to_owned(),
            ],
            attempt_id: None,
            system: "x86_64-linux".to_owned(),
            success: false,
        };

        assert_eq!(
            &result_to_comment(&result),
            "Failure on x86_64-linux [(full log)](https://logs.nix.ci/?key=nixos/nixpkgs.2345&attempt_id=none)

<details><summary>Partial log (click to expand)</summary><p>

```
make[2]: Entering directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1/readline'
make[2]: Nothing to be done for 'install'.
make[2]: Leaving directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1/readline'
make[1]: Nothing to be done for 'install-target'.
make[1]: Leaving directory '/private/tmp/nix-build-gdb-8.1.drv-0/gdb-8.1'
removed '/nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1/share/info/bfd.info'
post-installation fixup
strip is /nix/store/5a88zk3jgimdmzg8rfhvm93kxib3njf9-cctools-binutils-darwin/bin/strip
patching script interpreter paths in /nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1
/nix/store/pcja75y9isdvgz5i00pkrpif9rxzxc29-gdb-8.1
```
</p></details>

"
        );
    }
}
