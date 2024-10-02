TALK "Please, take a photo of the QR Code and send to me."
HEAR doc as QRCODE
text = GET "doc-"  + doc + ".pdf"

IF text THEN

    text = "Based on this document, answer the person's questions:\n\n" + text
    SET CONTEXT text 
    SET ANSWER MODE "document"
    TALK "Document ${doc} loaded. You can ask me anything about it."

ELSE
    TALK "Document was not found, please try again."
END IF