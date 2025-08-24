PARAM name AS string          LIKE "Abreu Silva"                DESCRIPTION "Required full name of the individual."
PARAM birthday AS date        LIKE "23/09/2001"                 DESCRIPTION "Required birth date of the individual in DD/MM/YYYY format."
PARAM email AS string         LIKE "abreu.silva@example.com"    DESCRIPTION "Required email address for contact purposes."
PARAM personalid AS integer   LIKE "12345678900"                DESCRIPTION "Required Personal ID number of the individual (only numbers)."
PARAM address AS string       LIKE "Rua das Flores, 123 - SP"   DESCRIPTION "Required full address of the individual."

DESCRIPTION  "This is a the enrollment process, called when the user wants to enrol. Once all information is collected, confirm the details and inform them that their enrollment request has been successfully submitted. Provide a polite and professional tone throughout the interaction."

SAVE "enrollments.csv", id, name, birthday, email, personalid, address
