

let text = GET "default.gbdrive/default.pdf"
let resume = LLM "Build table resume with deadlines, dates and actions: " + text

SET_BOT_MEMORY "resume" resume
