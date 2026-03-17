"""
Mutable artifact: Support ticket classifier.
This file is the ONLY thing the EGRI loop may modify.

Must export: classify(text: str) -> str
Valid labels: "billing", "account", "bug"
"""


def classify(text: str) -> str:
    text = text.lower()
    if any(w in text for w in ("pay", "charge", "refund", "invoice", "bill", "subscription", "receipt", "discount", "cancel")):
        return "billing"
    elif any(w in text for w in ("account", "log", "password", "username", "email", "profile", "merge", "two-factor", "authentication")):
        return "account"
    else:
        return "bug"
