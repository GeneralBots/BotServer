TALK "What is the case number?"
HEAR caseNumber
text = GET "case.pdf"
text = "Based on this document, answer the person's questions:\n\n" + text
SET CONTEXT text 
SET ANSWER MODE "direct"

TALK "Case ${caseNumber} loaded. You can ask me anything about the case or request a summary in any way you need."
