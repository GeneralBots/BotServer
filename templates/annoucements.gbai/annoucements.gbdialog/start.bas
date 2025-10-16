TALK "Olá, pode me perguntar sobre qualquer coisa..."
let text = GET "default.gbdrive/default.pdf"
let resume = LLM "Say Hello and present a a resume from " + text
TALK resume
SET_CONTEXT "Este é o documento que você deve usar para responder dúvidas: " + text
return true;
