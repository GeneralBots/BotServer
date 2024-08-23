
REM SET SCHEDULE "1 * * * * *"

billing = FIND "Billing"

REM Consumo Mensal de barras.

data = SELECT SUM(effectiveUnitPrice) as Value, MONTH(usageDate)+'/'+YEAR(usageDate) from billing GROUP BY MONTH(date), YEAR(date)
img  = CHART "timseries", data
SEND FILE img, "Consumo Mensal"

REM Categoria do Produto

data = SELECT SUM(effectiveUnitPrice) as Value, meterCategory from billing GROUP BY meterCategory
img  = CHART "donut", data
SEND FILE img, "Categoria do Produto"

REM Sub Categoria do Produto

data = SELECT SUM(effectiveUnitPrice) as Value, meterSubCategory from billing GROUP BY meterCategory
img  = CHART "donut", data
SEND FILE img, "Subcategoria do Produto"

REM Nome do Produto (Resumido)
REM productName

REM Região do Recurso
REM resourceLocation

REM Grupo do Recurso
REM resourceGroup
 

REM Consumo Mensal de barras (Envio individual para cada cliente)

customers = FIND "Customers"
FOR EACH c IN customers
data = SELECT SUM(effectiveUnitPrice) as Value, MONTH(usageDate)+'/'+YEAR(usageDate) from billing GROUP BY MONTH(date), YEAR(date) WHERE customerId = c.id
img  = CHART "timseries", data
SEND FILE img, "Consumo Mensal"
END FOR

