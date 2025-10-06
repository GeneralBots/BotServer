# 📚 **BASIC LEARNING EXAMPLES - FORMAT Function**

## 🎯 **EXAMPLE 1: BASIC CONCEPT OF FORMAT FUNCTION**

```
**BASIC CONCEPT:**
FORMAT FUNCTION - Value formatting

**LEVEL:**
☒ Beginner ☐ Intermediate ☐ Advanced

**LEARNING OBJECTIVE:**
Understand how to format numbers, dates, and text

**CODE EXAMPLE:**
```basic
10 NUMBER = 1234.56
20 TEXT$ = "John"
30 DATE$ = "2024-03-15 14:30:00"
40 
50 PRINT FORMAT(NUMBER, "n")      ' 1234.56
60 PRINT FORMAT(NUMBER, "F")      ' 1234.56
70 PRINT FORMAT(TEXT$, "Hello @!") ' Hello John!
80 PRINT FORMAT(DATE$, "dd/MM/yyyy") ' 15/03/2024
```

**SPECIFIC QUESTIONS:**
- What's the difference between "n" and "F"?
- What does "@" mean in text?
- How to format dates in Brazilian format?

**PROJECT CONTEXT:**
I need to display data in a nicer way

**EXPECTED RESULT:**
Values formatted according to the pattern

**PARTS I DON'T UNDERSTAND:**
- When to use each type of formatting
- How it works internally
```

---

## 🛠️ **EXAMPLE 2: NUMERIC FORMATTING**

```
**BASIC CONCEPT:**
NUMBER FORMATTING

**LEVEL:**
☒ Beginner ☐ Intermediate ☐ Advanced

**LEARNING OBJECTIVE:**
Learn to format numbers as currency and with separators

**CODE EXAMPLE:**
```basic
10 VALUE = 1234567.89
20 
30 PRINT "Standard: "; FORMAT(VALUE, "n")        ' 1234567.89
40 PRINT "Decimal: "; FORMAT(VALUE, "F")         ' 1234567.89
45 PRINT "Integer: "; FORMAT(VALUE, "f")         ' 1234567
50 PRINT "Percentage: "; FORMAT(0.856, "0%")     ' 86%
60 
70 ' Formatting with locale
80 PRINT "Dollar: "; FORMAT(VALUE, "C2[en]")     ' $1,234,567.89
90 PRINT "Real: "; FORMAT(VALUE, "C2[pt]")       ' R$ 1.234.567,89
100 PRINT "Euro: "; FORMAT(VALUE, "C2[fr]")      ' €1,234,567.89
```

**SPECIFIC QUESTIONS:**
- What does "C2[pt]" mean?
- How to change decimal places?
- Which locales are available?

**PROJECT CONTEXT:**
Multi-currency financial system

**EXPECTED RESULT:**
Numbers formatted according to regional standards

**PARTS I DON'T UNDERSTAND:**
- Syntax of complex patterns
- Differences between locales
```

---

## 📖 **EXAMPLE 3: EXPLAINING FORMAT COMMAND**

```
**COMMAND:**
FORMAT - Formats values

**SYNTAX:**
```basic
RESULT$ = FORMAT(VALUE, PATTERN$)
```

**PARAMETERS:**
- VALUE: Number, date or text to format
- PATTERN$: String with formatting pattern

**SIMPLE EXAMPLE:**
```basic
10 PRINT FORMAT(123.45, "n")           ' 123.45
20 PRINT FORMAT("Mary", "Ms. @")       ' Ms. Mary
```

**PRACTICAL EXAMPLE:**
```basic
10 INPUT "Name: "; NAME$
20 INPUT "Salary: "; SALARY
30 INPUT "Birth date: "; BIRTH_DATE$
40 
50 PRINT "Record:"
60 PRINT "Name: "; FORMAT(NAME$, "!")           ' UPPERCASE
70 PRINT "Salary: "; FORMAT(SALARY, "C2[en]")   ' $1,234.56
80 PRINT "Birth: "; FORMAT(BIRTH_DATE$, "MM/dd/yyyy")
```

**COMMON ERRORS:**
- Using wrong pattern for data type
- Forgetting it returns string
- Formatting date without correct format

**BEGINNER TIP:**
Test each pattern separately before using in project

**SUGGESTED EXERCISE:**
Create a bank statement with professional formatting
```

---

## 🎨 **EXAMPLE 4: DATE AND TIME FORMATTING**

```
**BASIC CONCEPT:**
DATE AND TIME FORMATTING

**LEVEL:**
☐ Beginner ☒ Intermediate ☐ Advanced

**LEARNING OBJECTIVE:**
Learn all date formatting patterns

**CODE EXAMPLE:**
```basic
10 DATE$ = "2024-03-15 14:30:25"
20 
30 PRINT "Brazilian: "; FORMAT(DATE$, "dd/MM/yyyy")        ' 15/03/2024
40 PRINT "Complete: "; FORMAT(DATE$, "dd/MM/yyyy HH:mm")   ' 15/03/2024 14:30
50 PRINT "US: "; FORMAT(DATE$, "MM/dd/yyyy")               ' 03/15/2024
60 PRINT "International: "; FORMAT(DATE$, "yyyy-MM-dd")    ' 2024-03-15
70 
80 PRINT "24h Time: "; FORMAT(DATE$, "HH:mm:ss")           ' 14:30:25
90 PRINT "12h Time: "; FORMAT(DATE$, "hh:mm:ss tt")        ' 02:30:25 PM
100 PRINT "Long date: "; FORMAT(DATE$, "dd 'of' MMMM 'of' yyyy")
```

**SPECIFIC QUESTIONS:**
- What's the difference between HH and hh?
- How to show month name?
- What is "tt"?

**PROJECT CONTEXT:**
Scheduling system and reports

**EXPECTED RESULT:**
Dates formatted according to needs

**PARTS I DON'T UNDERSTAND:**
- All formatting codes
- How milliseconds work
```

---

## 🏆 **EXAMPLE 5: COMPLETE PROJECT - BANK STATEMENT**

```
# BASIC PROJECT: FORMATTED BANK STATEMENT

## 📝 DESCRIPTION
System that generates bank statement with professional formatting

## 🎨 FEATURES
- [x] Currency formatting
- [x] Date formatting
- [x] Value alignment

## 🧩 CODE STRUCTURE
```basic
10 ' Customer data
20 NAME$ = "Carlos Silva"
30 BALANCE = 12567.89
40 
50 ' Transactions
60 DIM DATES$(3), DESCRIPTIONS$(3), AMOUNTS(3)
70 DATES$(1) = "2024-03-10 09:15:00" : DESCRIPTIONS$(1) = "Deposit" : AMOUNTS(1) = 2000
80 DATES$(2) = "2024-03-12 14:20:00" : DESCRIPTIONS$(2) = "Withdrawal" : AMOUNTS(2) = -500
90 DATES$(3) = "2024-03-14 11:30:00" : DESCRIPTIONS$(3) = "Transfer" : AMOUNTS(3) = -150.50
100 
110 ' Header
120 PRINT FORMAT("BANK STATEMENT", "!")
130 PRINT "Customer: "; FORMAT(NAME$, "&")
140 PRINT "Date: "; FORMAT("2024-03-15 08:00:00", "dd/MM/yyyy HH:mm")
150 PRINT STRING$(40, "-")
160 
170 ' Transactions
180 FOR I = 1 TO 3
190   FORMATTED_DATE$ = FORMAT(DATES$(I), "dd/MM HH:mm")
200   FORMATTED_AMOUNT$ = FORMAT(AMOUNTS(I), "C2[en]")
210   
220   PRINT FORMATTED_DATE$; " - "; 
230   PRINT DESCRIPTIONS$(I);
240   PRINT TAB(30); FORMATTED_AMOUNT$
250 NEXT I
260 
270 ' Balance
280 PRINT STRING$(40, "-")
290 PRINT "Balance: "; TAB(30); FORMAT(BALANCE, "C2[en]")
```

## 🎯 LEARNINGS
- Currency formatting with locale
- Date formatting
- Composition of multiple formats

## ❓ QUESTIONS TO EVOLVE
- How to perfectly align columns?
- How to format negative numbers in red?
- How to add more locales?
```

---

## 🛠️ **EXAMPLE 6: TEXT FORMATTING**

```
**BASIC CONCEPT:**
STRING/TEXT FORMATTING

**LEVEL:**
☒ Beginner ☐ Intermediate ☐ Advanced

**LEARNING OBJECTIVE:**
Learn to use placeholders in text

**CODE EXAMPLE:**
```basic
10 NAME$ = "Mary"
20 CITY$ = "são paulo"
21 COUNTRY$ = "BRAZIL"
22 AGE = 25
30 
40 PRINT FORMAT(NAME$, "Hello @!")              ' Hello Mary!
50 PRINT FORMAT(NAME$, "Welcome, @")            ' Welcome, Mary
60 PRINT FORMAT(CITY$, "City: !")               ' City: SÃO PAULO
70 PRINT FORMAT(CITY$, "City: &")               ' City: são paulo
80 PRINT FORMAT(COUNTRY$, "Country: &")         ' Country: brazil
90 
100 ' Combining with numbers
110 PRINT FORMAT(NAME$, "@ is ") + FORMAT(AGE, "n") + " years old"
120 ' Mary is 25 years old
```

**SPECIFIC QUESTIONS:**
- What's the difference between @, ! and &?
- Can I use multiple placeholders?
- How to escape special characters?

**PROJECT CONTEXT:**
Personalized report generation

**EXPECTED RESULT:**
Dynamic texts formatted automatically

**PARTS I DON'T UNDERSTAND:**
- Placeholder limitations
- How to mix different types
```

---

## 📚 **EXAMPLE 7: PRACTICAL EXERCISES**

```
# EXERCISES: PRACTICING WITH FORMAT

## 🎯 EXERCISE 1 - BASIC
Create a program that formats product prices.

**SOLUTION:**
```basic
10 DIM PRODUCTS$(3), PRICES(3)
20 PRODUCTS$(1) = "Laptop" : PRICES(1) = 2500.99
30 PRODUCTS$(2) = "Mouse" : PRICES(2) = 45.5
40 PRODUCTS$(3) = "Keyboard" : PRICES(3) = 120.75
50 
60 FOR I = 1 TO 3
70   PRINT FORMAT(PRODUCTS$(I), "@: ") + FORMAT(PRICES(I), "C2[en]")
80 NEXT I
```

## 🎯 EXERCISE 2 - INTERMEDIATE
Make a program that shows dates in different formats.

**SOLUTION:**
```basic
10 DATE$ = "2024-12-25 20:00:00"
20 
30 PRINT "Christmas: "; FORMAT(DATE$, "dd/MM/yyyy")
40 PRINT "US: "; FORMAT(DATE$, "MM/dd/yyyy")
50 PRINT "Dinner: "; FORMAT(DATE$, "HH'h'mm")
60 PRINT "Formatted: "; FORMAT(DATE$, "dd 'of' MMMM 'of' yyyy 'at' HH:mm")
```

## 🎯 EXERCISE 3 - ADVANCED
Create a school report card system with formatting.

**SOLUTION:**
```basic
10 NAME$ = "ana silva"
20 AVERAGE = 8.75
21 ATTENDANCE = 0.92
30 REPORT_DATE$ = "2024-03-15 10:00:00"
40 
50 PRINT FORMAT("SCHOOL REPORT CARD", "!")
60 PRINT "Student: "; FORMAT(NAME$, "&")
70 PRINT "Date: "; FORMAT(REPORT_DATE$, "dd/MM/yyyy")
80 PRINT "Average: "; FORMAT(AVERAGE, "n")
90 PRINT "Attendance: "; FORMAT(ATTENDANCE, "0%")
```

## 💡 TIPS
- Always test patterns before using
- Use PRINT to see each formatting result
- Combine simple formats to create complex ones
```

---

## 🎨 **EXAMPLE 8: COMPLETE REFERENCE GUIDE**

```markdown
# FORMAT FUNCTION - COMPLETE GUIDE

## 🎯 OBJECTIVE
Format numbers, dates and text professionally

## 📋 SYNTAX
```basic
RESULT$ = FORMAT(VALUE, PATTERN$)
```

## 🔢 NUMERIC FORMATTING
| Pattern | Example | Result |
|---------|---------|--------|
| "n" | `FORMAT(1234.5, "n")` | 1234.50 |
| "F" | `FORMAT(1234.5, "F")` | 1234.50 |
| "f" | `FORMAT(1234.5, "f")` | 1234 |
| "0%" | `FORMAT(0.85, "0%")` | 85% |
| "C2[en]" | `FORMAT(1234.5, "C2[en]")` | $1,234.50 |
| "C2[pt]" | `FORMAT(1234.5, "C2[pt]")` | R$ 1.234,50 |

## 📅 DATE FORMATTING
| Code | Meaning | Example |
|------|---------|---------|
| yyyy | 4-digit year | 2024 |
| yy | 2-digit year | 24 |
| MM | 2-digit month | 03 |
| M | 1-2 digit month | 3 |
| dd | 2-digit day | 05 |
| d | 1-2 digit day | 5 |
| HH | 24h hour 2-digit | 14 |
| H | 24h hour 1-2 digit | 14 |
| hh | 12h hour 2-digit | 02 |
| h | 12h hour 1-2 digit | 2 |
| mm | 2-digit minute | 05 |
| m | 1-2 digit minute | 5 |
| ss | 2-digit second | 09 |
| s | 1-2 digit second | 9 |
| tt | AM/PM | PM |
| t | A/P | P |

## 📝 TEXT FORMATTING
| Placeholder | Function | Example |
|-------------|----------|---------|
| @ | Insert original text | `FORMAT("John", "@")` → John |
| ! | Text in UPPERCASE | `FORMAT("John", "!")` → JOHN |
| & | Text in lowercase | `FORMAT("John", "&")` → john |

## ⚠️ LIMITATIONS
- Dates must be in "YYYY-MM-DD HH:MM:SS" format
- Very large numbers may have issues
- Supported locales: en, pt, fr, de, es, it
```

These examples cover from basic to advanced applications of the FORMAT function! 🚀