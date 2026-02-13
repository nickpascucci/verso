"""
This example demonstrates the --annotations-file flag.

The 'demo/greeting' annotation below conflicts with one stored in
stored-annotations.json. The extracted version wins.
"""

def greet():
    #@<demo/greeting
    print("This is the NEW version extracted from source.")
    #>@

#@<demo/source-only
def farewell():
    print("Goodbye!")
#>@
