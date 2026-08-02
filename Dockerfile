FROM python:3.14-slim

ARG WHEEL
ARG VERSION
ARG REVISION
ARG SOURCE

LABEL org.opencontainers.image.title="valens"
LABEL org.opencontainers.image.description="An app for tracking your health and training progress."
LABEL org.opencontainers.image.licenses=AGPL-3.0-only
LABEL org.opencontainers.image.version=$VERSION
LABEL org.opencontainers.image.revision=$REVISION
LABEL org.opencontainers.image.source=$SOURCE

ENV GUNICORN_WORKERS=2
ENV GUNICORN_THREADS=1
ENV GUNICORN_TIMEOUT=120
ENV PYTHONUNBUFFERED=1
ENV VALENS_CONFIG=/app/config.py
ENV ALEMBIC_CONFIG=/etc/valens/alembic.ini

RUN pip install --no-cache-dir gunicorn

COPY ${WHEEL} /tmp/
RUN pip install --no-cache-dir /tmp/*.whl && rm -rf /tmp/*.whl

COPY alembic.ini /etc/valens/

RUN mkdir -p /app
RUN useradd --create-home --shell /usr/sbin/nologin valens && chown -R valens:valens /app

WORKDIR /app
USER valens

EXPOSE 8000

CMD sh -c '\
  if [ ! -f "$VALENS_CONFIG" ]; then \
    valens config -d "$(dirname "$VALENS_CONFIG")" --database "$PWD/valens.db"; \
  fi; \
  exec gunicorn valens:app \
    --bind 0.0.0.0:8000 \
    --workers ${GUNICORN_WORKERS} \
    --threads ${GUNICORN_THREADS} \
    --timeout ${GUNICORN_TIMEOUT} \
    --access-logfile - \
    --error-logfile - \
'
