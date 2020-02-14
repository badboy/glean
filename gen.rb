#!/usr/bin/env ruby
# encoding: utf-8

require 'erb'

OUTPUT = "glean-core/preview/user_metrics.rs"
HEADER = <<-EOF
use glean_core::{metrics::StringMetric, CommonMetricData, Lifetime};

EOF

METRIC_TMPL = <<-EOF
#[allow(non_upper_case_globals)]
static <%= var_name %>: Lazy<StringMetric> = Lazy::new(|| {
    StringMetric::new(CommonMetricData {
        name: "<%= name %>".into(),
        category: "<%= category %>".into(),
        send_in_pings: vec!["prototype".into()],
        lifetime: Lifetime::Ping,
        disabled: false,
        dynamic_label: None,
    })
});

EOF

FOOTER = <<-EOF
fn set_them_all(glean: &glean_core::Glean) {
<%- names.each do |name| -%>
    <%= name %>.set(glean, "val");
<%- end -%>
}
EOF

n = (ARGV.first || 10).to_i
$stderr.puts "Generating #{n} metrics in #{OUTPUT}"
File.open(OUTPUT, 'w') do |file|
  file.write HEADER

  names = n.times.map do |i|
    name = "metric_#{i}"
    var_name = "Metric#{i}"
    category = "data"

    data = ERB.new(METRIC_TMPL).result_with_hash({
      name: name,
      var_name: var_name,
      category: category
    })
    file.write(data)

    var_name
  end

  data = ERB.new(FOOTER, nil, '-').result_with_hash({
    names: names
  })
  file.write(data)
end
