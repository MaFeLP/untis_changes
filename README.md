# Untis Changes
## Running
```shell
docker pull ghcr.io/mafelp/untis_changes:main
docker run -d \
    -p 8080:80 \
    -e 'TZ=Europe/Berlin' \
    -e 'UNTIS_HOST=example.untis.com' \
    -e 'UNTIS_SCHOOL=ab1234' \
    ghcr.io/mafelp/untis_changes:main
```

## Building
```shell
docker build -t ghcr.io/mafelp/untis_changes:main .
```
