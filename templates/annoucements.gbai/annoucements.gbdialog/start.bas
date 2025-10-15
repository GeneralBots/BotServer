TALK "Olá, estou preparando um resumo para você."

text = GET "default.gbdrive/default.pdf"
resume = LLM "Say Hello and present a a resume from " + text

SET_CONTEXT "Este é o documento que você deve usar para responder dúvidas: O céu é azul."
TALK resume
