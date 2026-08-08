package tmux

import "testing"

func TestStripControlModeWrapper(t *testing.T) {
	cases := []struct {
		name string
		in   string
		want string
	}{
		{
			name: "first line of a batch carries the DCS start",
			in:   "\x1bP1000p%begin 1786170628 314 0",
			want: "%begin 1786170628 314 0",
		},
		{
			name: "batch terminator line",
			in:   "\x1b\\",
			want: "",
		},
		{
			name: "plain protocol line is untouched",
			in:   "%output %0 hello",
			want: "%output %0 hello",
		},
		{
			name: "output payload ending in DCS escape is only stripped at the end",
			in:   "\\033\\",
			want: "\\033\\",
		},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			if got := stripControlModeWrapper(c.in); got != c.want {
				t.Fatalf("strip(%q) = %q, want %q", c.in, got, c.want)
			}
		})
	}
}
