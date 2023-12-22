## Problem

With this tool you can backup big volumes of data and split it into multiple fixed-size files. Why to split? Because a single huge monolithic file is extremely hard to manage, especially when using network-mounted filesystems (e.g. DavFS, SSHFS, NFS, etc). For most cloud providers, uploading, say, a 500G file is a challenge: it may reject it an with error, or the upload may be interrupted in the middle (with or without error), or any other things may happen depending on the phase of the moon. For example, Google Drive does not work well over DavFS with +2G files, and YandexDisk starts to misbehave around 1G.

The tool compresses the input data stream with XZ algorithm and encrypts using [authenticated encryption](https://en.wikipedia.org/wiki/Authenticated_encryption) providing both confidentiality and integrity. The AES-128-GCM is currently used as it performs super fast on modern CPUs and also gives high resistance. It perfectly fits cloud infrastructures where security is a regulatory requirement. With this type of encryption scheme, any attacker's attempt to modify the encrypted data (without decrypting) will be detected.

Finally, additional assurance is maintained since the integrity of resulting files is verified right after each backup, so one can be sure that when the backups as needed, they are readable and contain the exact source data.

## Usage samples

#### Example to backup data coming from stdin into files

`tar cf - /my/disk | ./bigarchiver --backup --buf-size 256 --auth "My Full Name" --auth-every 32 --pass mysecret --compress-level 6 --split-size 1024 --out-template /path/to/files%%%%%%`

#### Example to restore data from files to stdout:

`./bigarchiver --restore --check-free-space /my --buf-size 256 --pass mysecret --config /path/to/files000000.cfg | tar xf - /my/disk`

#### Example to verify the backup files without actual restore:

`./bigarchiver --check --buf-size 256 --pass mysecret --config /path/to/files000000.cfg`

## Command line option reference

| Option                                                   | Meaning |
|----------------------------------------------------------|---------|
| `--backup, --restore, --check` | select mode of operation (only one at a time) |
| `--buf-size <size_MB>` | buffer size to use when reading or writing (see _Memory usage_ section below for details) |
| `--pass <password>` | password for encryption or decryption<br/>**WARNING:** it's impossible to restore the archive if password is lost! |
| `--auth <auth_string>` | any arbitrary public authentication string that will be embedded into to archive; can be someone's name or passport ID, or company name; it's not kept in secret, but an attacker won't be able to impersonate this string |
| `--auth-every <size_MB>` | how frequent to insert the authentication string; any reasonable value around dozens/hundreds of megabytes is ok |
| `--compress-level <level>` | set XZ compression preset, valid values are from 0 to 9 (see _Memory usage_ section below for details); set to 6 if unsure |
| `--split-size <size_MB>` | output chunk size to split to |
| `--out-template <template>` | full path how to name output files; any sequence of '%' characters will accept sequence number; if no '%' sequence is found, or it appears more than once, the error will be returned |
| `--config <config>` | full path to config file left from a previous successful backup operation |
| `--check-free-space <path>` | check free space available on the indicated filesystem before restore |
| `--no-check` | for backup mode, don't do integrity check _after_ backup creation; for restore mode, don't do integrity check _before_ restoring |

## Memory usage

The tool allows control of how much memory will be used. On the one hand, the more memory it uses, the faster will be the operation. On the other hand, using too much memory will put other processes' memory pages into swap that may not be desired. So in the absence of one-size-fits-all approach, the option `--buf-size` should be used. The overall memory consumption can be _roughly_ estimated as follows:

`MEM_USAGE_APPRX = BUF_SIZE + XZ_CONSUMPTION`

where _XZ_CONSUMPTION_ is additional memory intensively swallowed by XZ compressor/decompressor module, which, in turn, can be estimated like this:

| XZ level | Compressor,  MB | Decompressor, MB |
|---|---|---|
| 0 | 5 | 1 |
| 1 | 10 | 2 |
| 2 | 20 | 3 |
| 3 | 30 | 5 |
| 4 | 50 | 5 |
| 5 | 100 | 10 |
| 6 | 100 | 10 |
| 7 | 190 | 20 |
| 8 | 370 | 30 |
| 9 | 680 | 65 |

## Q & A

Q: why is this tool needed if one can use something like `tar | xz | openssl | split`?

A: those kind of "shell" approach would require an significant amount of accompanying code, mainly to verify the correctness of the written result. Not to mention the portability problems of different shells in different systems.

Q: why the Authenticated encryption is used, and not just plain old AES (or any other proven symmetric encryption)?

A: basic symmetric encryption provides only confidentiality assurance (meaning unauthorized persons cannot read the data), but it lacks authenticity (meaning no unauthorized modifications can go undetected, even if it's just a dumb corruption of data). This is where AEAD encryption comes into scene.

Q: is the encryption hardware accelerated?

A: yes, as long as your CPU support AES-NI instructions.

Q: which compression level should I use?

A: it depends how much memory and CPU one can devote to backup process. Setting too low levels makes sense when input data is of high randomness (e.g. it already consists of some archive files, so trying to compress them will drain CPU power for nothing), or a machine has very little memory available. Setting too high levels is only useful when the output size is critical and the destination storage is expensive. All in all, for majority of cases levels of 4-6 is the best approach.

Q: if during the backup process something goes wrong, e.g. something cannot be written on the filesystem?

A: the process stops with non-zero exit code leaving everything partially written, i.e. no cleanup is done. Proper cleanup will be probably implemented in the future.

Q: how is the encryption key produced from the string password given?

A: password-based key derivation function PBKDF2-HMAC-SHA256 is used with 100k iterations

## Disclaimer

Although the tool is abundant with tests and coded in Rust, it's written by a human and may contain errors. The author has not responsibility on lost data of any production servers in case something goes wrong.
