# ldenv

Uses [dotenvy](https://github.com/allan2/dotenvy) but uses mode files

By default the mode is local and it will load in order

- .env.local
- .env

and so .env.local has priority

By default if no mode is provided on the command line, it will get the mode from the environment variable `MODE`

you can specify a different env variable to get the default mode via `-n <env variable name>`

And you can specify the mode directly via `-m <mode>`


for example with with `ldenv -m production env`

it will load the following in order

- .env.production.local
- .env.production
- .env

and execute the command `env`
