
let text = GET "default.gbdrive/default.pdf"
let resume = LLM "Resume this document, in a table (DO NOT THINK) no_think: " + text

SET_BOT_MEMORY "resume", resume
