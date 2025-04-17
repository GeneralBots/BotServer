TALK "What is the case number?"
HEAR cod
text = GET "case-" + cod + ".pdf"

IF text THEN 

    text = "Based on this document, answer the person's questions:\n\n" + text
    SET CONTEXT text 
    SET ANSWER MODE "document"
    TALK "Case ${cod} loaded. You can ask me anything about the case or request a summary in any way you need."

ELSE
    TALK "The case was not found, please try again."
END IF