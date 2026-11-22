# Shopping Sessions build and runtime.
# The scorers are linux/amd64 (Intel Xeon). They build this file natively, so
# no FROM line pins a platform: `make build` builds for linux/amd64 anywhere.
FROM rust:1.85-bookworm@sha256:e51d0265072d2d9d5d320f6a44dde6b9ef13653b035098febd68cce8fa7c0bc4 AS compiler
COPY pipeline/clean.rs pipeline/csv.rs pipeline/events.rs /tmp/
RUN rustc -O --edition 2024 --crate-type cdylib /tmp/events.rs -o /events.so
RUN rustc -O --edition 2024 /tmp/clean.rs -o /clean

COPY pipeline/attribute.rs /tmp/attribute.rs
RUN rustc -O --edition 2024 /tmp/attribute.rs -o /attribute

FROM python:3.12-slim-bookworm@sha256:935c3c5b12357a7e10caa9c1e6ca5692344d97cf229832ef584be2cec9fc03df
COPY --from=compiler /attribute /app/attribute
COPY --from=compiler /events.so /app/events.so
COPY --from=compiler /clean /app/clean

# The scorer runs the container as uid 1000; give that uid a name and a home.
RUN useradd --uid 1000 --no-create-home --home-dir /scratch app && install -d -o 1000 -g 1000 /scratch

COPY requirements.txt /app/requirements.txt
RUN pip install --no-cache-dir -r /app/requirements.txt

COPY pipeline/ /app/pipeline/
COPY run.sh /app/run.sh
RUN chmod +x /app/run.sh

# At run time the root filesystem is read-only, the user is 1000:1000, there is
# no network, /data is read-only, and /output and /scratch are the only writable
# places. run.sh points every temp and cache directory at /scratch.
ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1

WORKDIR /app
CMD ["/app/run.sh"]
