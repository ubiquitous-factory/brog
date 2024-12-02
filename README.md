# brog
A gitops client for bootc. 

## introduction

[bootc](https://containers.github.io/bootc/) is a transactional, in-place operating system update mechanism based on OCI/Docker container images. It's a fantastic technology that is capable of increasing deployment velocity in edge scenarios by an order of magnitude. However the operational tooling for managing the role out of updates in a safe and consistent manner still needs to be built out by the organisations using the technologies. 

[brog](https://mehal.tech/brog) aims to address this by offering a gitops model for managing updates and coupled with [clos](https://mehal.tech/clos) offer simple yet robust solutions for managing role out of updates to large edge estates.  

## usage

1. In your host operating system disable automatic updates.
    ```
    systemctl disable --now bootc-fetch-apply-updates.timer
    ```

1. In your bootc image definition copy the `brog` executable from the release container.

    ```
    FROM quay.io/mehal_tech/brog as build
    ...
    COPY --from=build /app/brog /usr/bin
    ```

1. Create a `brog.service` file based on the sample and update the `ENDPOINT` and `SCHEDULE` as required.
   ```
    [Service]
    ...
    Environment=ENDPOINT=https://yourserver/yourproject/brog.yaml
    Environment=SCHEDULE="every 5 seconds"
   ``` 

1. Copy the  `brog.service` into your bootc definition and enable the service.
    ```
    COPY brog.service /etc/systemd/system
    RUN systemctl enable brog.service
    ```

## environment variables

|Value|Description|Required|Example|Default|
|---|---|---|---|---|
|ENDPOINT|The location of the brog config file|yes|https://github.com/you/yourproject/brog.yaml|None|
|SCHEDULE|CRON and English format schedule definition|yes| "1/4 * * * * *" or "every 4 seconds"|None|
|BROG_PATH|Additional $PATH configuration for brog|no|"/host/usr/bin"|"/host/usr/bin"|
|LOG_LEVEL|Sets logging level for the service|no|debug|info|
|CLOS_TOKEN|Required if you need canary deployments or private repo support|no|See [CLOS Service Config](https://mehal.tech/clos/brogconfig)|None|

## Roadmap

|Item|Complete|
|---|---|
|Open http endpoint|&#x2611;|
|Send Machine Identifier in request|&#x2610;|
|Integrate with secrets managment systems|&#x2610;|
|Private GitHub Repo|&#x2610;|
|Private Gitlab Repo|&#x2610;|
|Canary Support from [CLOS](https://mehal.tech/clos)|&#x2610;|
|Container Based Deployment|&#x2610;|