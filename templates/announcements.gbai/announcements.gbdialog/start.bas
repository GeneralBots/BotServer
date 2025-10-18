let resume = GET_BOT_MEMORY ("resume")

TALK resume

let text = GET "default.gbdrive/default.pdf"
SET_CONTEXT "Este é o documento que você deve usar para responder dúvidas: " + text
TALK "Olá, pode me perguntar sobre qualquer coisa destas circulares..."
