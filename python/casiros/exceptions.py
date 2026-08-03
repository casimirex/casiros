"""Exceptions raised by the CASIROS Python client."""


class CasirosError(Exception):
    """Base exception for all CASIROS client errors."""


class CasirosApiError(CasirosError):
    """Raised when the CASIROS API returns an error response.

    Attributes:
        status_code: HTTP status code returned by the API.
        message: Human-readable error message.
    """

    def __init__(self, status_code: int, message: str) -> None:
        self.status_code = status_code
        self.message = message
        super().__init__(f"CASIROS API error {status_code}: {message}")
