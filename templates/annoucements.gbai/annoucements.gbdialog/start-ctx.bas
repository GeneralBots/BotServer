TALK "Welcome to General Bots! What is your name?"
HEAR name
TALK "Hello, " + name

text = GET "default.pdf"
SET_CONTEXT text

resume = LLM "Build a resume from " + text
