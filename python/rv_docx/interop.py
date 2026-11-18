"""Optional interoperability with python-docx; native bytes/save do not need it."""
import io


def to_docx(data):
    try:
        import docx
    except ImportError as error:
        raise ImportError("Install rv-docx-rs[interop] to use to_docx()") from error
    return docx.Document(io.BytesIO(data))
