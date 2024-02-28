 - A duration of zero now translates to an, effectively, infinite duration
 - Gasta container not displays Greet.md on login.
 - Removed Sasta trim_trailing_slash() in path layer, meaning /api/test does not equal /api/test/ anymore due to causing swagger-ui infinite redirect error
 - Add simple Casta implementation using HTMX hosted on `/display/<uuid>` in sasta. No setup required on client :)

File mounted in Docker image as Greet.md will be displayed and rendered on login. Mounting a directory to /files/ allows static file hosting, which are inaccessible without login in.
