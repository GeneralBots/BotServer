PARAM nome AS string LIKE "João Silva" DESCRIPTION "Required full name of the individual."
PARAM datanasc AS date LIKE "23/09/2001" DESCRIPTION "Required birth date of the individual in DD/MM/YYYY format."
PARAM email AS string LIKE "joao.silva@example.com" DESCRIPTION "Required email address for contact purposes."
PARAM cpf AS integer LIKE "12345678900" DESCRIPTION "Required CPF number of the individual (only numbers)."
PARAM rg AS integer LIKE "12345678" DESCRIPTION "Required RG or CNH number of the individual (only numbers)."
PARAM orgaorg AS string LIKE "SSP-SP" DESCRIPTION "Required issuing authority of the individual's RG or CNH."
PARAM dataemite AS date LIKE "15/08/2007" DESCRIPTION "Required issue date of the individual's RG or CNH in DD/MM/YYYY format."
PARAM ender AS string LIKE "Rua das Flores, 123, São Paulo, SP" DESCRIPTION "Required full address of the individual."
PARAM nomealuno AS string LIKE "Ana Silva" DESCRIPTION "Required full name of the student for enrollment."
PARAM cpfaluno AS integer LIKE "98765432100" DESCRIPTION "Required CPF number of the student (only numbers)."
PARAM datanascaluno AS date LIKE "07/03/2010" DESCRIPTION "Required birth date of the student in DD/MM/YYYY format."
PARAM rgaluno AS integer LIKE "87654321" DESCRIPTION "Required RG number of the student (only numbers)."
PARAM orgaoaluno AS string LIKE "SSP-SP" DESCRIPTION "Required issuing authority of the student's RG or CNH."
PARAM emissaoaluno AS date LIKE "10/05/2015" DESCRIPTION "Required issue date of the student's RG or CNH in DD/MM/YYYY format."
PARAM respfinaluno AS string LIKE "Maria Oliveira" DESCRIPTION "Required full name of the financial responsible party for the student."

DESCRIPTION  "This is a the enrollment process, called when the user wants to enrol. Once all information is collected, confirm the details and inform them that their enrollment request has been successfully submitted. Provide a polite and professional tone throughout the interaction."

SAVE "enrollments.csv", id, from, nome, datanasc, email, cpf, rg, orgaorg, dataemite, ender, nomealuno, cpfaluno, datanascaluno
    