' General Bots Copyright (c) pragmatismo.cloud. All rights reserved. Licensed under the AGPL-3.0. 
' Rules from http://jsfiddle.net/roderick/dym05hsy

talk "How many installments do you want to pay your Credit?"
hear installments

If installments > 60 Then
    talk "The maximum number of payments is 60"
Else
    talk "What is the amount requested?"
    hear amount
    
    If amount > 100000 Then
        talk "We are sorry, we can only accept proposals bellow 100k"
    Else
        
        talk "What is the best due date?"
        hear dueDate
        
        interestRate = 0
        adjustment = 0
        
        If installments < 12 Then
            interestRate = 1.60
            adjustment = 0.09748
        End If
        
        If installments > 12 And installments < 18 Then
            interestRate = 1.66
            adjustment = 0.06869
        End If
        
        If installments > 18 And installments < 36 Then
            interestRate = 1.64
            adjustment = 0.05397
        End If
        
        If installments > 36 And installments < 48 Then
            interestRate = 1.62
            adjustment = 0.03931
        End If
        
        If installments > 48 And installments < 60 Then
            interestRate = 1.70
            adjustment = 0.03270
        End If
        
        If installments = 60 Then
            interestRate = 1.79
            adjustment = 0.02916
        End If
        
        If installments > 60 Then
            talk "The maximum number of payments is 60"
        End If
        
        nInstallments = parseInt(installments)
        vamount = parseFloat(amount)
        initialPayment = vamount * 0.3 ' 30% of the value
        tac = 800
        adjustment = 1.3
        
        totalValue = amount - initialPayment + tac
        paymentValue = totalValue * adjustment
        finalValue = paymentValue * nInstallments + initialPayment
        
        talk "Congratulations! Your credit analysis is **done**:"
        talk "First payment: **" + initialPayment + "**"
        talk "Payment value: **" + paymentValue + "**"
        talk "Interest Rate: **" + interestRate + "%**"
        talk "Total Value: **" + totalValue + "**"
        talk "Final Value: **" + finalValue + "**"
        
        

    End If
End If