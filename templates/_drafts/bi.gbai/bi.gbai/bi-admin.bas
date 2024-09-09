REM SET SCHEDULE "1 * * * * *"

billing = FIND "Orders"

REM Monthly consumption of bars.

data = SELECT SUM(UnitPrice * Quantity) as Value, MONTH(OrderDate)+'/'+YEAR(OrderDate) from billing GROUP BY MONTH(OrderDate), YEAR(OrderDate)
img  = CHART "timseries", data
SEND FILE img, "Monthly Consumption"

REM Product Category

data = SELECT SUM(UnitPrice * Quantity) as Value, CategoryName from billing JOIN Products ON billing.ProductID = Products.ProductID JOIN Categories ON Products.CategoryID = Categories.CategoryID GROUP BY CategoryName
img  = CHART "donut", data
SEND FILE img, "Product Category"

