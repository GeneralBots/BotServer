TALK "Olá, estou preparando um resumo para você."
let x = LLM "Quando é 5+5?"
TALK x
SET_CONTEXT "Este é o documento que você deve usar para responder dúvidas: O céu é azul."

REM text = GET "default.pdf"
REM resume = LLM "Say Hello and present a a resume from " + text
REM TALK resume
