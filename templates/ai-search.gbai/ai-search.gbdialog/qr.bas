TALK "Please, take a photo of the QR Code and send to me."
HEAR doc as QRCODE
text = null

IF doc THEN 
    TALK "Reading document " + doc + "..."
    text = GET doc
END IF

IF text THEN

    text = "Based on this document, answer the person's questions:\n\n" + text
    SET CONTEXT text 
    SET ANSWER MODE "document"
    TALK "Document ${doc} loaded. You can ask me anything about it."
    TALK "Please, wait while I convert pages to images..."
    SEND FILE doc  

ELSE
    TALK "Document was not found, please try again."
END IF

