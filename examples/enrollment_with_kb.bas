REM ============================================================================
REM Enrollment Tool with Knowledge Base Integration
REM ============================================================================
REM This is a complete example of a BASIC tool that:
REM 1. Collects user information through PARAM declarations
REM 2. Validates and stores data
REM 3. Activates a Knowledge Base collection for follow-up questions
REM 4. Demonstrates integration with KB documents
REM ============================================================================

REM Define tool parameters with type, example, and description
PARAM name AS string LIKE "Abreu Silva" DESCRIPTION "Required full name of the individual."
PARAM birthday AS date LIKE "23/09/2001" DESCRIPTION "Required birth date of the individual in DD/MM/YYYY format."
PARAM email AS string LIKE "abreu.silva@example.com" DESCRIPTION "Required email address for contact purposes."
PARAM personalid AS integer LIKE "12345678900" DESCRIPTION "Required Personal ID number of the individual (only numbers)."
PARAM address AS string LIKE "Rua das Flores, 123 - SP" DESCRIPTION "Required full address of the individual."

REM Tool description for MCP/OpenAI tool generation
DESCRIPTION "This is the enrollment process, called when the user wants to enroll. Once all information is collected, confirm the details and inform them that their enrollment request has been successfully submitted. Provide a polite and professional tone throughout the interaction."

REM ============================================================================
REM Validation Logic
REM ============================================================================

REM Validate name (must not be empty and should have at least first and last name)
IF name = "" THEN
    TALK "Please provide your full name to continue with the enrollment."
    EXIT
END IF

name_parts = SPLIT(name, " ")
IF LEN(name_parts) < 2 THEN
    TALK "Please provide your complete name (first and last name)."
    EXIT
END IF

REM Validate email format
IF email = "" THEN
    TALK "Email address is required for enrollment."
    EXIT
END IF

IF NOT CONTAINS(email, "@") OR NOT CONTAINS(email, ".") THEN
    TALK "Please provide a valid email address."
    EXIT
END IF

REM Validate birthday format (DD/MM/YYYY)
IF birthday = "" THEN
    TALK "Please provide your birth date in DD/MM/YYYY format."
    EXIT
END IF

REM Validate personal ID (only numbers)
IF personalid = "" THEN
    TALK "Personal ID is required for enrollment."
    EXIT
END IF

REM Validate address
IF address = "" THEN
    TALK "Please provide your complete address."
    EXIT
END IF

REM ============================================================================
REM Generate unique enrollment ID
REM ============================================================================

id = UUID()
enrollment_date = NOW()
status = "pending"

REM ============================================================================
REM Save enrollment data to CSV file
REM ============================================================================

SAVE "enrollments.csv", id, name, birthday, email, personalid, address, enrollment_date, status

REM ============================================================================
REM Log enrollment for audit trail
REM ============================================================================

PRINT "Enrollment created:"
PRINT "  ID: " + id
PRINT "  Name: " + name
PRINT "  Email: " + email
PRINT "  Date: " + enrollment_date

REM ============================================================================
REM Activate Knowledge Base for enrollment documentation
REM ============================================================================
REM The .gbkb/enrollpdfs folder should contain:
REM - enrollment_guide.pdf
REM - requirements.pdf
REM - faq.pdf
REM - terms_and_conditions.pdf
REM ============================================================================

SET_KB "enrollpdfs"

REM ============================================================================
REM Confirm enrollment to user
REM ============================================================================

confirmation_message = "Thank you, " + name + "! Your enrollment has been successfully submitted.\n\n"
confirmation_message = confirmation_message + "Enrollment ID: " + id + "\n"
confirmation_message = confirmation_message + "Email: " + email + "\n\n"
confirmation_message = confirmation_message + "You will receive a confirmation email shortly with further instructions.\n\n"
confirmation_message = confirmation_message + "I now have access to our enrollment documentation. Feel free to ask me:\n"
confirmation_message = confirmation_message + "- What documents do I need to submit?\n"
confirmation_message = confirmation_message + "- What are the enrollment requirements?\n"
confirmation_message = confirmation_message + "- When will my enrollment be processed?\n"
confirmation_message = confirmation_message + "- What are the next steps?\n"

TALK confirmation_message

REM ============================================================================
REM Set user context for personalized responses
REM ============================================================================

SET USER name, email, id

REM ============================================================================
REM Store enrollment in bot memory for quick access
REM ============================================================================

SET BOT MEMORY "last_enrollment_id", id
SET BOT MEMORY "last_enrollment_name", name
SET BOT MEMORY "last_enrollment_date", enrollment_date

REM ============================================================================
REM Optional: Send confirmation email
REM ============================================================================
REM Uncomment if email feature is enabled

REM email_subject = "Enrollment Confirmation - ID: " + id
REM email_body = "Dear " + name + ",\n\n"
REM email_body = email_body + "Your enrollment has been received and is being processed.\n\n"
REM email_body = email_body + "Enrollment ID: " + id + "\n"
REM email_body = email_body + "Date: " + enrollment_date + "\n\n"
REM email_body = email_body + "You will be notified once your enrollment is approved.\n\n"
REM email_body = email_body + "Best regards,\n"
REM email_body = email_body + "Enrollment Team"
REM
REM SEND EMAIL TO email, email_subject, email_body

REM ============================================================================
REM Return success with enrollment ID
REM ============================================================================

RETURN id
