## Problem

With this tool you can backup big volumes of data and split it into multiple fixed-size files. Why to split? Because a single huge monolitic file is extremely hard to manage, especially when using network-mounted filesystems (e.g. DavFS). For most cloud providers, uploading, say, a 500G file is a challenge: it may reject it an with error, or the upload may be interrupted in the middle (with or without error), or any other things may happen depending on the phase of the moon. For example, Google Drive does not behave well over DavFS for +2G files, and YandexDisk starts to misbehave around 1G.

The tool compresses the input data stream with XZ algorithm and encrypts using [authenticated encryption](https://en.wikipedia.org/wiki/Authenticated_encryption) providing both confidentiality and integrity. The AES-128 GCM is currently used as it performs super fast on modern CPUs and also gives high resistence. It perfectly fits cloud infrastructures where security is a regulatory requirement. With this type of encryption scheme, any attacker's attempt to modify the encrypted data (without decrypting) will be detected.

Finally, additional assurance is maintaned since the integrity of resulting files is verified right after each backup, so one can be sure that when the backups as needed, they are readable and contain the exact source data.

# Usage samples

## Example to backup data coming from stdin into files

`tar cf - /my/disk | ./bigarchiver --backup --buf-size 256 --auth "My Full Name" --auth-every 32 --pass mysecret --compress-level 6 --split-size 1024 --out-template /path/to/files%%%%%%`

## Example to restore data from files to stdout:

`./bigarchiver --restore --check-free-space /my ] --buf-size 256 --pass mysecret --config /path/to/files000000.cfg | tar xf - /my/disk`

## Example to verify the backup files without actual restore:

`./bigarchiver --check --buf-size 256 --pass mysecret --config /path/to/files000000.cfg`

# Command line option reference

<table>
<thead>

<tr>
<th width="500px">Option</th>
<th width="1000px">Meaning</th>
</tr>

</thead>
<tbody>

<tr>
<td>--backup, --restore, <em>--check</em></td><td>select mode of <b>operation</b> (<i>only</i> one at a time)</td>
</tr>

</tbody>
</table>

| Option                               | Meaning |
|------------------------|---------|
| --buf-size <size_MB> | buffer size to use when reading or writing (see _Memory usage_ section below for details) |
| `--pass <password>` | password for encryption or decryption<br/>**WARNING:** it's impossible to restore the archive if password is lost! |
| `--auth <auth_string>` | any arbitrary public authentication string that will be embedded into to archive; can be someone's name or passport ID, or company name; it's not kept in secret, but an attacker won't be able to impersonate this string |
| `--auth-every <size_MB>` | how frequent to insert the authentication string; any reasonble value around dozens of megabytes is ok |
| `--compress-level <level>` | set XZ compression preset, valid values are from 0 to 6 (see _Compression preset_ section below for details |
| `--split-size <size_MB>` | output chunk size to split to |
| `--out-template <template>` | full path how to name output files; any sequence of '%' characters will accept sequence number; if no '%' sequence is found, or it appears more than ones, the error will be returned |

