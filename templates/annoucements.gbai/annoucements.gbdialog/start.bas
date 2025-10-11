TALK "Welcome to General Bots!"
HEAR name
TALK "Hello, " + name

text = GET "default.pdf"
SET CONTEXT text

resume = LLM "Build a resume from " + text
