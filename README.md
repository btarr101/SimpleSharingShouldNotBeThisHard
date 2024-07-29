# SimpleSharingShouldNotBeThisHard.com

I just want to share files without creating an account, without size restrictions and prompts for me to upgrade to a premium service, and without whoever I want to share em' with needing to download them.

## Description

This is an axum web application that allows for temporary uploading of files. It does this by assigning a v7 uuid to files uploaded with the expiration timestamp of the file. It groups files that expire within the same hour in folders in s3 - where a batch job routinely cleans up expired files.

## Getting Started

### Dependencies

- Rust
- Shuttle
- TODO: docs on setting up a MINIO container (or config for using memory w/ opendal for local testing)

### Executing program

```
cargo shuttle run
```

## Help

TODO

## Authors

Contributors names and contact info

Benjamin Tarr

## License

This project is licensed under the [TODO NAME HERE] License - see the LICENSE.md file for details

## Acknowledgments

TODO
