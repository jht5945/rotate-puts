# rotate-puts

Rotate outputs from std in or assigned file

Usage:
```shell
command | rotate-puts
```

Use `mkfifo`:
```shell
mkfido test-fifo
rotate-puts --file test-fifo
command > test-fifo
```
