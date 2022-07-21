Install rust if you don't have it already:

```curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh```

To install:

```cargo install --path . --root ~```

or

```make install```


This will place the binary in ```$HOME/bin```

Usage:
```
$ grok --help
grok 0.4.3
James Birchfield <jbirchfield@demeterlogistics.com>
Streams Cloudwatch Logs

USAGE:
    grok [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
        --list       lists log groups only
    -n, --nocolor    disable color highlighting
    -V, --version    Prints version information

OPTIONS:
    -e, --end <end>            optional end date (i.e. now, 1 hour ago)
    -g, --groups <groups>      CSV of all groups to read (or all:<filter>)
    -l, --level <level>        filter to a certain log level [default: ALL]  [possible values: ALL, TRACE, DEBUG, WARN,
                               INFO, ERROR]
    -p, --pattern <pattern>    Optional pattern to match
    -r, --region <region>      optional region [default: us-east-1]
    -s, --start <start>        optional start date (i.e. 1 hour ago)

```
Example usage:

```
grok -g /aws/lambda/data-prod-PutStandardOrder
```
or

```
grok -g all:lambda  -s "2h ago" -e "1h ago"
```
