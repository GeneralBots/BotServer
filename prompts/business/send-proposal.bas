PARAM to AS STRING
PARAM template AS STRING
PARAM opportunity AS STRING

company = QUERY "SELECT Company FROM Opportunities WHERE Id = ${opportunity}"

doc = FILL template

' Generate email subject and content based on conversation history
subject = REWRITE "Based on this ${history}, generate a subject for a proposal email to ${company}"
contents = REWRITE "Based on this ${history}, and ${subject}, generate the e-mail body for ${to}, signed by ${user}, including key points from our proposal"

' Add proposal to CRM
CALL "/files/upload", ".gbdrive/Proposals/${company}-proposal.docx", doc
CALL "/files/permissions", ".gbdrive/Proposals/${company}-proposal.docx", "sales-team", "edit"

' Record activity in CRM
CALL "/crm/activities/create", opportunity, "email_sent", { 
  "subject": subject,
  "description": "Proposal sent to " + company,
  "date": NOW()
}

' Send the email
CALL "/comm/email/send", to, subject, contents, doc
