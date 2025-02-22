# brog
A gitops client for bootc based on the [bootc management service recommendations](https://github.com/containers/bootc/blob/main/docs/src/building/management-services.md).

## introduction

[bootc](https://containers.github.io/bootc/) is a transactional, in-place operating system update mechanism based on OCI/Docker container images. It's a fantastic technology that is capable of increasing deployment velocity in edge scenarios by an order of magnitude. However the operational tooling for managing the role out of updates in a safe and consistent manner still needs to be built out by the organisations using the technologies. 

[brog](https://mehal.tech/brog) aims to address this by offering a gitops model for managing updates and coupled with [clos](https://mehal.tech/clos) offer simple yet robust solutions for managing role out of updates to large edge estates.  



[![Build](https://github.com/mehal-tech/brog/actions/workflows/build-test.yaml/badge.svg)](https://github.com/mehal-tech/brog/actions/workflows/build-test.yaml)

## usage

1. In your bootc image definition copy the `brog` executable from the release container.

    ```dockerfile
    # Reference the brog distribution container
    FROM ghcr.io/ubiquitous-factory/brog as build
    ...
    # Disable automatic updates
    RUN systemctl disable bootc-fetch-apply-updates.timer

    # Copy the file from the distribution container
    COPY --from=build /vendor/fedora41/brog /usr/bin

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
|BROG_PATH|Additional $PATH configuration for brog to find bootc|no|"/usr/local/bin"|"/usr/bin:/usr/sbin"|
|LOG_LEVEL|Sets logging level for the service|no|debug|info|
|CLOS_TOKEN|Required if you need canary deployments or private repo support|no|See [CLOS Service Config](https://mehal.tech/clos/brogconfig)|None|
|SERVICE_NAME|Configurable service name if you are writing a backend for brog|no|myservicename|projects|

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
