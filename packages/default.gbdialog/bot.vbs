<%
'****************************************************************************
'                                               ( )_  _                       
'    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     
'   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   
'   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| ( ) |( (_) )  
'   | ,__/'(_)  `\__,_)`\__  |(_) (_) (_)`\__,_)`\__)(_)(____/(_) (_)`\___/'  
'   | |                ( )_) |                                                
'   (_)                 \___/'                                                
'                                                                             
' General Bots Copyright (c) Pragmatismo.io. All rights reserved.             
' Licensed under the AGPL-3.0.                                                
'
' This BASIC file is based on this JavaScript file by Rodrigo Ruotolo:
'   -> http://jsfiddle.net/roderick/dym05hsy
'                                                                              
' According to our dual licensing model, this program can be used either      
' under the terms of the GNU Affero General Public License, version 3,        
' or under a proprietary license.                                             
'                                                                             
' The texts of the GNU Affero General Public License with an additional       
' permission and of our proprietary license can be found at and               
' in the LICENSE file you have received along with this program.              
'                                                                             
' This program is distributed in the hope that it will be useful,             
' but WITHOUT ANY WARRANTY, without even the implied warranty of              
' MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                
' GNU Affero General Public License for more details.                         
'                                                                             
' "General Bots" is a registered trademark of Pragmatismo.io.                 
' The licensing of the program under the AGPLv3 does not imply a              
' trademark license. Therefore any rights, title and interest in              
' our trademarks remain entirely with us.                                     
'                                                                             
'****************************************************************************

talk ("How many installments do you want to pay your Credit?")
installments = hear ()

if installments > 60 then
    talk ("The maximum number of payments is 60")
else
 	talk ("What is the amount requested?")
	ammount = hear ()

	if ammount >100000 then
		talk ("We are sorry, we can only accept proposals bellow 100k")
	else

		talk ("What is the best due date?")
		dueDate = hear ()

		interestRate = 0
		adjustment = 0

		if installments < 12 then
			interestRate = 1.60
			adjustment = 0.09748
		end if

		if installments > 12 and installments < 18 then
			interestRate = 1.66
			adjustment = 0.06869
		end if

		if installments > 18 and installments < 36 then
			interestRate = 1.64
			adjustment = 0.05397
		end if

		if installments > 36 and installments < 48 then
			interestRate = 1.62
			adjustment = 0.03931
		end if

		if installments > 48 and installments < 60 then
			interestRate = 1.70
			adjustment = 0.03270
		end if

		if installments = 60 then
			interestRate = 1.79
			adjustment = 0.02916
		end if

		if installments > 60 then
			talk ("The maximum number of payments is 60")
		end if


		' TODO: This must be reviewed in terms of financing logic.
		
		nInstallments = parseInt(installments)
		vAmmount = parseFloat(ammount)
		initialPayment = parseFloat(vAmmount) * 0.3 ' 30% of the value
		tac = 800
		adjustment = 1.3

		totalValue = ammount - initialPayment + tac    
		paymentValue = totalValue * adjustment
		finalValue = paymentValue * nInstallments + initialPayment

		talk("Congratulations! Your credit analysis is **done**:") 
		talk("First payment: **" + initialPayment + "**")
		talk("Payment value: **" + paymentValue + "**")
		talk("Interest Rate: **" + interestRate + "%**")
		talk("Total Value: **" + totalValue + "**")
		talk("Final Value: **" + finalValue + "**")
		
	end if
end if
%>