#! /bin/bash

main() {
    canvas_submit_to=""
    canvas_submittable=0

    echo "Selected $1 to upload, searching for commit with upload data"

    dir_md="$(dirname "$1")"
    base_md="$(basename "$1")"

    cd "$dir_md" || exit

    eval "$(get_canvas_info HEAD)"

    if [ "$canvas_submittable" != 0 ]; 
    then 
        echo "Submitting to $canvas_submit_to"

        assignment_name="$(basename "$canvas_submit_to")"
        commit_hash="$(git rev-parse --short HEAD)"

        filename="$(pwd)/$assignment_name-$commit_hash.pdf"
        html_filename="$(mktemp --suffix=".html")"

        echo "Rendering Markdown to HTML"
        render_html "$base_md" > "$html_filename"

        echo "Rendering HTML to PDF"
        chrome --headless --disable-gpu --print-to-pdf="./$filename" "$html_filename"

        echo "Uploading PDF to Canvas"
        submit_to_canvas "$canvas_submit_to" "$filename"
    fi
}

function get_canvas_info() {
    git show -s --format=%b "$(git rev-parse "$1")" |
    while IFS= read -r line; do
        echo "debug: Reading line $line"
        if echo "$line" | grep -qi "^submit-to: "; then 
            value=$(echo $line | sed 's/submit-to: //i')

            if [[ "$value" == previous* ]]; then 
                get_canvas_info "$1~"
            else 
                echo "canvas_submit_to='$value'; canvas_submittable=1;"
            fi
        fi
    done
}

HTML_RENDERER_FILENAME="$(mktemp)"

function download_html_renderer() {
    curl -o "$HTML_RENDERER_FILENAME" -L "https://github.com/chlohal/gh-canvas/releases/download/v0.01/render-html"
    chmod +x "$HTML_RENDERER_FILENAME"
}

function render_html() {
    if [ ! -f "./render-html-precompiled" ] 
    then 
        download_html_renderer; 
    fi

    "$HTML_RENDERER_FILENAME" "$@"
}


function curl_canvas_api() {
    curl -L -H "Authorization: Bearer $CANVAS_TOKEN" "$CANVAS_BASE_URL/$1" "${@:2}"
}

function upload_canvas_file() {
    filename="$1"
    course_id="$2"
    assignment_id="$3"

    base_filename="$(basename "$filename")"
    file_size="$(stat --printf="%s" "$filename")"

    upload_curl_command="$(curl_canvas_api "/api/v1/courses/$course_id/assignments/$assignment_id/submissions/self/files" \
     -F "name=$base_filename" \
     -F "size=$file_size" | 
         jq -r '"curl \(.upload_url)\( [.upload_params | to_entries[] | " -F \(.key)=\(.value)" ] | join(" ") )"'
     ) -F 'file=@$filename'"

     eval "$upload_curl_command" | jq '.id'

}

function get_course_id_for_name() {
    course_code="$1"

    curl_canvas_api "/api/v1/courses?enrollment_type=student&enrollment_state=active" |
        jq '.[] | { id, course_code } | select(.course_code | test("'"$course_code"'"; "i")) | .id'
}

function get_assignment_for_name() {
    course_id="$1"
    assignment_name="$2"

    curl_canvas_api "/api/v1/courses/$course_id/assignments" |
        jq '.[] | { id, name } | select(.name | test("'"$assignment_name"'"; "i")) | .id'
}

function submit_to_canvas() {
    to_submit_to="$1"
    submission_file="$2"

    course_code="$(dirname "$to_submit_to")"
    assignment_name="$(basename "$to_submit_to")"

    course_id="$(get_course_id_for_name "$course_code")"

    assignment_id="$(get_assignment_for_name "$course_id" "$assignment_name")"

    canvas_file_id="$(upload_canvas_file "$submission_file" "$course_id" "$assignment_id")"

    curl_canvas_api "/api/v1/courses/$course_id/assignments/$assignment_id/submissions"  \
    -F "submission[submission_type]=online_upload" \
    -F "submission[file_ids][]=$canvas_file_id" |
        jq -r '.preview_url'

}

main "$@"