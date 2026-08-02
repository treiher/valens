from datetime import timedelta

PERMANENT_SESSION_LIFETIME = timedelta(weeks=52)
# Refreshing the session cookie on every response would allow a response to a request of a
# previous session to overwrite the cookie of a newly created session. A session therefore expires
# `PERMANENT_SESSION_LIFETIME` after the sign-in instead of after the last request.
SESSION_REFRESH_EACH_REQUEST = False
SQLITE_FOREIGN_KEY_SUPPORT = True
USERNAME_LOGIN_ENABLED = True
