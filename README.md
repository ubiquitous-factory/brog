# brog
A gitops client for bootc

## environment variables

|Value|Description|Required|Example|
|---|---|---|---|
|ENDPOINT|The location of the CLOS config file|yes|https://github.com/you/yourproject/clos.yaml|
|SCHEDULE|CRON and English format schedule definition|yes| "1/4 * * * * *" or "every 4 seconds"|
|BROG_PATH|Additional $PATH configuration for brog|no|"/host/usr/bin"|
|CLOS_TOKEN|Required if you need canary deployments or private repo support|no|See [CLOS Service Config](https://mehal.tech/clos/brogconfig)| 