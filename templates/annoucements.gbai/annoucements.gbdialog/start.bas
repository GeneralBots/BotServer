TALK "Olá, pode me perguntar sobre qualquer coisa..."

text = GET "default.gbdrive/default.pdf"
resume = LLM "Say Hello and present a a resume from " + text
TALK resume

SET_CONTEXT "Este é o documento que você deve usar para responder dúvidas: " + text
