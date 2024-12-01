# brog
A gitops client for bootc

## environment variables

|Value|Description|Required|Example|Default|
|---|---|---|---|---|
|ENDPOINT|The location of the CLOS config file|yes|https://github.com/you/yourproject/clos.yaml|None|
|SCHEDULE|CRON and English format schedule definition|yes| "1/4 * * * * *" or "every 4 seconds"|None|
|BROG_PATH|Additional $PATH configuration for brog|no|"/host/usr/bin"|"/host/usr/bin"|
|LOG_LEVEL|Sets logging level for the service|no|debug|info|
|CLOS_TOKEN|Required if you need canary deployments or private repo support|no|See [CLOS Service Config](https://mehal.tech/clos/brogconfig)|None|