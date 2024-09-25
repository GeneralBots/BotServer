TALK "Hello, could you please tell me your full name?"
HEAR name

TALK "What is your contact email?"
HEAR email

TALK "What is your contact phone number?"
HEAR phone

TALK "Job title?"
HEAR job

TALK "Which company will you be representing?"
HEAR company

TALK "Are you a freelancer?"
HEAR freelancer AS BOOLEAN

TALK "Which city do you plan to attend the event in?"
HEAR city AS "São Paulo", "Rio de Janeiro"

SAVE "event-guests.xlsx", name, email, phone, job, company, freelancer
TALK "Thank you!"