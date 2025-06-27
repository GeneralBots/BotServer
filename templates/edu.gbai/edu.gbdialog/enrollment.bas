PARAM name AS string LIKE "Abreu Silva" DESCRIPTION "Required full name of the individual."
PARAM birthday AS date LIKE "23/09/2001" DESCRIPTION "Required birth date of the individual in DD/MM/YYYY format."
PARAM email AS string LIKE "abreu.silva@example.com" DESCRIPTION "Required email address for contact purposes."
PARAM personalid AS integer LIKE "12345678900" DESCRIPTION "Required Personal ID number of the individual (only numbers)."
PARAM address AS string LIKE "Rua das Flores, 123, São Paulo, SP" DESCRIPTION "Required full address of the individual."

DESCRIPTION  "This is a the enrollment process, called when the user wants to enrol. Once all information is collected, confirm the details and inform them that their enrollment request has been successfully submitted. Provide a polite and professional tone throughout the interaction."

SAVE "enrollments.csv", id, name, birthday, email, personalid, address

IF name AND birthday AND email AND personalid AND address THEN
    TALK "Thank you for providing your details. We have successfully received your enrollment request."
    TALK "Here are the details you provided:"
    TALK "Name: ${name}"
    TALK "Birthday: ${birthday}"
    TALK "Email: ${email}"
    TALK "Personal ID: ${personalid}"
    TALK "Address: ${address}"
    TALK "Your enrollment request has been submitted successfully. We will contact you shortly with further information."
ELSE
    TALK "It seems that some required information is missing. Please ensure you provide all the necessary details to complete your enrollment."
END IF