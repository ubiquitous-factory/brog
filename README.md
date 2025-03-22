# brog
A gitops client for bootc based on the [bootc management service recommendations](https://github.com/containers/bootc/blob/main/docs/src/building/management-services.md).

## introduction

[bootc](https://containers.github.io/bootc/) is a transactional, in-place operating system update mechanism based on OCI/Docker container images. It's a fantastic technology that is capable of increasing deployment velocity in edge scenarios by an order of magnitude. However the operational tooling for managing the role out of updates in a safe and consistent manner still needs to be built out by the organisations using the technologies. 

[brog](https://mehal.tech/brog) aims to address this by offering a gitops model for managing updates and coupled with [clos](https://mehal.tech/clos) offer simple yet robust solutions for managing role out of updates to large edge estates.  



[![Build](https://github.com/mehal-tech/brog/actions/workflows/build-test.yaml/badge.svg)](https://github.com/mehal-tech/brog/actions/workflows/build-test.yaml) 
[![codecov](https://codecov.io/github/ubiquitous-factory/brog/graph/badge.svg?token=ANXI4rEspb)](https://codecov.io/github/ubiquitous-factory/brog)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/10182/badge)](https://www.bestpractices.dev/projects/10182)
## usage

1. In your bootc image definition copy the `brog` executable from the release container.

    ```dockerfile
    # Reference the brog distribution container
    FROM ghcr.io/ubiquitous-factory/brog as build
    ...
    # Disable automatic updates
    RUN systemctl disable bootc-fetch-apply-updates.timer

    # Copy the file from the distribution container
    COPY --from=build /vendor/fedora_41/brog /usr/bin

    # Create the service definition 
    # make sure to replace the `ENDPOINT` value with your gitops brog.yaml location
    COPY <<"EOT" /usr/lib/systemd/system/brog.service
    [Unit]
    Description=A bootc management service
    After=network.target

    [Service]
    Type=simple
    RemainAfterExit=yes
    ExecStart=/usr/bin/brog
    TimeoutStartSec=0
    Environment=ENDPOINT=https://YAML_HOST/brog.yaml
    Environment=SCHEDULE="every 120 seconds"

    [Install]
    WantedBy=default.target
    EOT

    # Enable the service
    # We prefer using systemctl over a manual symlink 
    RUN systemctl enable brog.service
    ```

## environment variables

|Value|Description|Required|Example|Default|
|---|---|---|---|---|
|ENDPOINT|The location of the brog config file|yes|https://github.com/you/yourproject/brog.yaml|None|
|SCHEDULE|CRON and English format schedule definition|yes| "1/4 * * * * *" or "every 4 seconds"|None|
|LOG_LEVEL|Sets logging level for the service|no|debug|info|
|SERVICE_KEY|Required if you need canary deployments or private repo support|no|See [CLOS Service Config](https://docs.mehal.tech/clos/osmanager)|None|
|SERVICE_SECRET|Required if you need canary deployments or private repo support|no|See [CLOS Service Config](https://docs.mehal.tech/clos/osmanager)|None|
|SERVICE_NAME|Configurable service name if you are writing a backend for brog|no|myservicename|projects|
|BIN_PATH|Additional $PATH configuration for brog to find bootc|no|"/usr/local/bin"|"/usr/bin:/usr/sbin"|
|CONFIG_PATH|location to write the latest commit file|no|"/etc/brog"|"/etc/brog"|

brog will look try and load environment variables from /etc/brog/.config.
Values in config do **not** override values specified in the service definition.

## development 

In debug mode brog will look for a `.env` file in the root of repository. 
It will required `ENDPOINT` and `SCHEDULE` populated.

## support matrix
|OS|Version|Architecture|Build Folder|
|---|---|---|---|
|Fedora|41|amd64, arm64|/vendor/fedora_41|

## roadmap

|Item|Complete|
|---|---|
|Open http endpoint|&#x2611;|
|Send Machine Identifier in request|&#x2611;|
|Integrate with secrets management systems|&#x2611;|
|Private GitHub Repo|&#x2611;|
|Private Gitlab Repo|&#x2610;|
|Canary Support from [CLOS](https://mehal.tech/clos)|&#x2611;|
|Container Based Deployment|&#x2611;|


## Conduct

We expect everyone who participates in this project in anyway to be friendly,
open-minded, and humble. We have a [Code of Conduct], and expect you to have
read it. If you have any questions or concerns, feel free to reach out to
Anton Whalley, antonwhalley@yahoo.com.

[Code of Conduct]: CODE_OF_CONDUCT.md

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or [apache.org/licenses/LICENSE-2.0](https://www.apache.org/licenses/LICENSE-2.0))
* MIT license ([LICENSE-MIT](LICENSE-MIT) or [opensource.org/licenses/MIT](https://opensource.org/licenses/MIT))

at your option.


### Contributions

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
If you want to contribute to `brog`, please read our [CONTRIBUTING notes].

[CONTRIBUTING notes]: CONTRIBUTING.md
