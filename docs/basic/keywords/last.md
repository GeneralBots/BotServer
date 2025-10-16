# 📚 **BASIC LEARNING EXAMPLES - LAST Function**

## 🎯 **EXAMPLE 1: BASIC CONCEPT OF LAST FUNCTION**

```
**BASIC CONCEPT:**
LAST FUNCTION - Extract last word

**LEVEL:**
☒ Beginner ☐ Intermediate ☐ Advanced

**LEARNING OBJECTIVE:**
Understand how the LAST function extracts the last word from text

**CODE EXAMPLE:**
```basic
10 PALAVRA$ = "The mouse chewed the clothes"
20 ULTIMA$ = LAST(PALAVRA$)
30 PRINT "Last word: "; ULTIMA$
```

**SPECIFIC QUESTIONS:**
- How does the function know where the last word ends?
- What happens if there are extra spaces?
- Can I use it with numeric variables?

**PROJECT CONTEXT:**
I'm creating a program that analyzes sentences

**EXPECTED RESULT:**
Should display: "Last word: clothes"

**PARTS I DON'T UNDERSTAND:**
- Why are parentheses needed?
- How does the function work internally?
```

---

## 🛠️ **EXAMPLE 2: SOLVING ERROR WITH LAST**

```
**BASIC ERROR:**
"Syntax error" when using LAST

**MY CODE:**
```basic
10 TEXTO$ = "Good day world"
20 RESULTADO$ = LAST TEXTO$
30 PRINT RESULTADO$
```

**PROBLEM LINE:**
Line 20

**EXPECTED BEHAVIOR:**
Show "world" on screen

**CURRENT BEHAVIOR:**
Syntax error

**WHAT I'VE TRIED:**
- Tried without parentheses
- Tried with different quotes
- Tried changing variable name

**BASIC VERSION:**
QBASIC with Rhai extension

**CORRECTED SOLUTION:**
```basic
10 TEXTO$ = "Good day world"
20 RESULTADO$ = LAST(TEXTO$)
30 PRINT RESULTADO$
```
```

---

## 📖 **EXAMPLE 3: EXPLAINING LAST COMMAND**

```
**COMMAND:**
LAST - Extracts last word

**SYNTAX:**
```basic
ULTIMA$ = LAST(TEXTO$)
```

**PARAMETERS:**
- TEXTO$: String from which to extract the last word

**SIMPLE EXAMPLE:**
```basic
10 FRASE$ = "The sun is bright"
20 ULTIMA$ = LAST(FRASE$)
30 PRINT ULTIMA$  ' Shows: bright
```

**PRACTICAL EXAMPLE:**
```basic
10 INPUT "Enter your full name: "; NOME$
20 SOBRENOME$ = LAST(NOME$)
30 PRINT "Hello Mr./Mrs. "; SOBRENOME$
```

**COMMON ERRORS:**
- Forgetting parentheses: `LAST TEXTO$` ❌
- Using with numbers: `LAST(123)` ❌
- Forgetting to assign to a variable

**BEGINNER TIP:**
Always use parentheses and ensure content is text

**SUGGESTED EXERCISE:**
Create a program that asks for a sentence and shows the first and last word
```

---

## 🎨 **EXAMPLE 4: COMPLETE PROJECT WITH LAST**

```
# BASIC PROJECT: SENTENCE ANALYZER

## 📝 DESCRIPTION
Program that analyzes sentences and extracts useful information

## 🎨 FEATURES
- [x] Extract last word
- [x] Count words
- [x] Show statistics

## 🧩 CODE STRUCTURE
```basic
10 PRINT "=== SENTENCE ANALYZER ==="
20 INPUT "Enter a sentence: "; FRASE$
30 
40 ' Extract last word
50 ULTIMA$ = LAST(FRASE$)
60 
70 ' Count words (simplified)
80 PALAVRAS = 1
90 FOR I = 1 TO LEN(FRASE$)
100   IF MID$(FRASE$, I, 1) = " " THEN PALAVRAS = PALAVRAS + 1
110 NEXT I
120 
130 PRINT
140 PRINT "Last word: "; ULTIMA$
150 PRINT "Total words: "; PALAVRAS
160 PRINT "Original sentence: "; FRASE$
```

## 🎯 LEARNINGS
- How to use LAST function
- How to count words manually
- String manipulation in BASIC

## ❓ QUESTIONS TO EVOLVE
- How to extract the first word?
- How to handle punctuation?
- How to work with multiple sentences?
```

---

## 🏆 **EXAMPLE 5: SPECIAL CASES AND TESTS**

```
**BASIC CONCEPT:**
SPECIAL CASES OF LAST FUNCTION

**LEVEL:**
☐ Beginner ☒ Intermediate ☐ Advanced

**LEARNING OBJECTIVE:**
Understand how LAST behaves in special situations

**CODE EXAMPLES:**
```basic
' Case 1: Empty string
10 TEXTO$ = ""
20 PRINT LAST(TEXTO$)  ' Result: ""

' Case 2: Single word only
30 TEXTO$ = "Sun"
40 PRINT LAST(TEXTO$)  ' Result: "Sun"

' Case 3: Multiple spaces
50 TEXTO$ = "Hello    World    "
60 PRINT LAST(TEXTO$)  ' Result: "World"

' Case 4: With tabs and newlines
70 TEXTO$ = "Line1" + CHR$(9) + "Line2" + CHR$(13)
80 PRINT LAST(TEXTO$)  ' Result: "Line2"
```

**SPECIFIC QUESTIONS:**
- What happens with empty strings?
- How does it work with special characters?
- Is it case-sensitive?

**PROJECT CONTEXT:**
I need to robustly validate user inputs

**EXPECTED RESULT:**
Consistent behavior in all cases

**PARTS I DON'T UNDERSTAND:**
- How the function handles whitespace?
- What are CHR$(9) and CHR$(13)?
```

---

## 🛠️ **EXAMPLE 6: INTEGRATION WITH OTHER FUNCTIONS**

```
**BASIC CONCEPT:**
COMBINING LAST WITH OTHER FUNCTIONS

**LEVEL:**
☐ Beginner ☒ Intermediate ☐ Advanced

**LEARNING OBJECTIVE:**
Learn to use LAST in more complex expressions

**CODE EXAMPLE:**
```basic
10 ' Example 1: With concatenation
20 PARTE1$ = "Programming"
30 PARTE2$ = " in BASIC"
40 FRASE_COMPLETA$ = PARTE1$ + PARTE2$
50 PRINT LAST(FRASE_COMPLETA$)  ' Result: "BASIC"

60 ' Example 2: With string functions
70 NOME_COMPLETO$ = "Maria Silva Santos"
80 SOBRENOME$ = LAST(NOME_COMPLETO$)
90 PRINT "Mr./Mrs. "; SOBRENOME$

100 ' Example 3: In conditional expressions
110 FRASE$ = "The sky is blue"
120 IF LAST(FRASE$) = "blue" THEN PRINT "The last word is blue!"
```

**SPECIFIC QUESTIONS:**
- Can I use LAST directly in IF?
- How to combine with LEFT$, RIGHT$, MID$?
- Is there a size limit for the string?

**PROJECT CONTEXT:**
Creating validations and text processing

**EXPECTED RESULT:**
Use LAST flexibly in different contexts

**PARTS I DON'T UNDERSTAND:**
- Expression evaluation order
- Performance with very large strings
```

---

## 📚 **EXAMPLE 7: PRACTICAL EXERCISES**

```
# EXERCISES: PRACTICING WITH LAST

## 🎯 EXERCISE 1 - BASIC
Create a program that asks for the user's full name and greets using only the last name.

**SOLUTION:**
```basic
10 INPUT "Enter your full name: "; NOME$
20 SOBRENOME$ = LAST(NOME$)
30 PRINT "Hello, Mr./Mrs. "; SOBRENOME$; "!"
```

## 🎯 EXERCISE 2 - INTERMEDIATE
Make a program that analyzes if the last word of a sentence is "end".

**SOLUTION:**
```basic
10 INPUT "Enter a sentence: "; FRASE$
20 IF LAST(FRASE$) = "end" THEN PRINT "Sentence ends with 'end'" ELSE PRINT "Sentence doesn't end with 'end'"
```

## 🎯 EXERCISE 3 - ADVANCED
Create a program that processes multiple sentences and shows statistics.

**SOLUTION:**
```basic
10 DIM FRASES$(3)
20 FRASES$(1) = "The sun shines"
30 FRASES$(2) = "The rain falls"
40 FRASES$(3) = "The wind blows"
50 
60 FOR I = 1 TO 3
70   PRINT "Sentence "; I; ": "; FRASES$(I)
80   PRINT "Last word: "; LAST(FRASES$(I))
90   PRINT
100 NEXT I
```

## 💡 TIPS
- Always test with different inputs
- Use PRINT for debugging
- Start with simple examples
```

---

## 🎨 **EXAMPLE 8: MARKDOWN DOCUMENTATION**

```markdown
# LAST FUNCTION - COMPLETE GUIDE

## 🎯 OBJECTIVE
Extract the last word from a string

## 📋 SYNTAX
```basic
RESULTADO$ = LAST(TEXTO$)
```

## 🧩 PARAMETERS
- `TEXTO$`: Input string

## 🔍 BEHAVIOR
- Splits string by spaces
- Returns the last part
- Ignores extra spaces at beginning/end

## 🚀 EXAMPLES
```basic
10 PRINT LAST("hello world")      ' Output: world
20 PRINT LAST("one word")         ' Output: word
30 PRINT LAST("  spaces  ")       ' Output: spaces
```

## ⚠️ LIMITATIONS
- Doesn't work with numbers
- Requires parentheses
- Considers only spaces as separators
```

These examples cover from the basic concept to practical applications of the LAST function, always focusing on BASIC beginners! 🚀