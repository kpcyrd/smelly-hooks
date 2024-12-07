# smelly-hooks

```
<Namarrgon> Nobody ever checks the .install script.
```

This is an experimental tool using symbolic execution to try to determine a pacman `.install` hook is only using reasonable binaries with reasonable flags.

## Reasoning

There are some operations that are considered reasonable and even though they may cause data loss or create local-privilege-escalation issues, they are considered not RCE-able:

- Changing file or permissions (chmod)
- Changing file owners or groups (chown/chgrp)
- Creating directories (mkdir)
- Creating empty files or changing their mtime (touch)
- Creating system users
- Deleting files or directories (rm/rmdir)
- Setting file capabilities on binaries (setcap)

This tool tries to flag "zero click" exploitation (not counting the start of the installation), with installation of the package leading to code execution with no further interaction (like manually running one of it's binaries). The package content itself may still still create files leading to code execution, like extracting a cronjob into the filesystem's configuration directory, this is considered out of scope.

## Bypasses

The following people found bypasses:

*Nobody yet, be the first! ✨*

This project follows a full-disclosure policy, if you find one please open a github issue.

## License

`GPL-3.0-or-later`
