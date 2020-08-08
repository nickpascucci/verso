# Example Markdown File

This is an example of a Verso / Recto Markdown file with annotations.

Within `example.py` there is an annotated region with the ID `examples/1`. Its contents are:

```
    print("Hello, World!")
    sys.exit(1)
```

There is also this annotated region with the ID `examples/2`:

```
if __name__ == "__main__":
    main()
```

Finally, this is the entire file:

```
import sys

def main():
    #@<examples/1
    print("Hello, World!")
    sys.exit(1)
    #>@examples/1

#@<examples/2
if __name__ == "__main__":
    main()
#>@examples/2

```

_Hopefully, that has some content!_
